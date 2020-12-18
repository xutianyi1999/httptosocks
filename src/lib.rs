#[macro_use]
extern crate log;

use std::convert::Infallible;
use std::io::Result;
use std::net::SocketAddr;
use std::os::raw::c_char;
use std::str::FromStr;

use async_socks5::connect;
use hyper::{Body, Client, Method, Request, Response, Server, Uri};
use hyper::client::HttpConnector;
use hyper::service::{make_service_fn, service_fn};
use hyper::upgrade::Upgraded;
use hyper_socks2::SocksConnector;
use log4rs::append::console::ConsoleAppender;
use log4rs::Config;
use log4rs::config::{Appender, Root};
use log4rs::encode::pattern::PatternEncoder;
use log::LevelFilter;
use tokio::net::TcpStream;
use tokio::runtime;

use crate::common::{StdResAutoConvert, StdResConvert, str_convert};

mod common;

type HttpClient = Client<SocksConnector<HttpConnector>>;

static mut IS_INIT: bool = false;

#[no_mangle]
pub extern fn start(proxy_addr: *const c_char, proxy_addr_len: u8, socks5_addr: *const c_char, socks5_addr_len: u8, threads: u8) {
    unsafe {
        if !IS_INIT {
            if let Err(e) = logger_init() {
                eprintln!("{}", e);
                return;
            }
            IS_INIT = true;
        }
    }

    let f = || {
        let proxy_addr = str_convert(proxy_addr, proxy_addr_len as usize)?;
        let socks5_addr = str_convert(socks5_addr, socks5_addr_len as usize)?;
        let threads = threads as usize;
        process(proxy_addr, socks5_addr, threads)
    };

    if let Err(e) = f() {
        error!("{}", e)
    }
}

fn process(proxy_addr: String, socks5_addr: String, threads: usize) -> Result<()> {
    let mut rt = runtime::Builder::new()
        .threaded_scheduler()
        .enable_all()
        .core_threads(threads)
        .build()?;

    let mut connector = HttpConnector::new();
    connector.enforce_http(false);

    let socks_proxy = SocksConnector {
        proxy_addr: Uri::from_str(&format!("socks5://{}", &socks5_addr)).res_auto_convert()?,
        auth: None,
        connector,
    };
    let client = Client::builder().build::<_, Body>(socks_proxy);

    rt.block_on(async move {
        let make_service = make_service_fn(move |_| {
            let client = client.clone();
            let socks5_addr = socks5_addr.clone();
            async move { Ok::<_, Infallible>(service_fn(move |req| proxy(client.clone(), req, socks5_addr.clone()))) }
        });

        let bind_addr = match SocketAddr::from_str(&proxy_addr) {
            Err(e) => {
                error!("{}", e);
                return;
            }
            Ok(addr) => addr
        };

        let server = Server::bind(&bind_addr).serve(make_service);
        info!("Listening on http://{}", proxy_addr);

        if let Err(e) = server.await {
            error!("Server error: {}", e);
        }
    });
    Ok(())
}

async fn proxy(client: HttpClient, req: Request<Body>, socks5_addr: String) -> hyper::Result<Response<Body>> {
    if Method::CONNECT == req.method() {
        tokio::spawn(async move {
            let uri = req.uri();

            let host = match uri.host() {
                Some(v) => v.to_string(),
                None => return
            };

            let port = match uri.port_u16() {
                Some(v) => v,
                None => return
            };

            match req.into_body().on_upgrade().await {
                Ok(upgraded) => {
                    let addr = (host, port);

                    if let Err(e) = tunnel(upgraded, addr, &socks5_addr).await {
                        error!("Server io error: {}", e);
                    };
                }
                Err(e) => error!("Upgrade error: {}", e),
            }
        });

        Ok(Response::new(Body::empty()))
    } else {
        client.request(req).await
    }
}

async fn tunnel(upgraded: Upgraded, addr: (String, u16), socks5_addr: &str) -> std::io::Result<()> {
    let mut stream = TcpStream::connect(socks5_addr).await?;
    connect(&mut stream, addr, None).await.res_convert(|_| "Connect socks5 server error".to_string())?;

    let amounts = {
        let (mut server_rd, mut server_wr) = tokio::io::split(stream);
        let (mut client_rd, mut client_wr) = tokio::io::split(upgraded);

        let client_to_server = tokio::io::copy(&mut client_rd, &mut server_wr);
        let server_to_client = tokio::io::copy(&mut server_rd, &mut client_wr);

        tokio::try_join!(client_to_server, server_to_client)
    };

    if let Err(e) = amounts {
        error!("Tunnel error: {}", e);
    }
    Ok(())
}

fn logger_init() -> Result<()> {
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("[Console] {d(%Y-%m-%d %H:%M:%S)} - {l} - {m}{n}")))
        .build();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(LevelFilter::Info))
        .res_auto_convert()?;

    log4rs::init_config(config).res_auto_convert()?;
    Ok(())
}