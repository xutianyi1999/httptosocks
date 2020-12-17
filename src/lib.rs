use std::convert::Infallible;
use std::io::Result;
use std::net::SocketAddr;
use std::os::raw::c_char;
use std::str::FromStr;

use async_socks5::connect;
use hyper::{Body, Client, http, Method, Request, Response, Server, Uri};
use hyper::client::HttpConnector;
use hyper::service::{make_service_fn, service_fn};
use hyper::upgrade::Upgraded;
use hyper_socks2::SocksConnector;
use tokio::net::TcpStream;
use tokio::runtime;

use crate::common::{StdResAutoConvert, StdResConvert, str_convert};

mod common;

type HttpClient = Client<SocksConnector<HttpConnector>>;

#[no_mangle]
pub extern fn start(proxy_addr: *const c_char, proxy_addr_len: u8, socks5_addr: *const c_char, socks5_addr_len: u8, threads: u8) {
    let proxy_addr = str_convert(proxy_addr, proxy_addr_len as usize).unwrap();
    let socks5_addr = str_convert(socks5_addr, socks5_addr_len as usize).unwrap();
    let threads = threads as usize;

    if let Err(e) = process(proxy_addr, socks5_addr, threads) {
        eprintln!("{}", e);
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
                eprintln!("{}", e);
                return;
            }
            Ok(addr) => addr
        };

        let server = Server::bind(&bind_addr).serve(make_service);
        println!("Listening on http://{}", proxy_addr);

        if let Err(e) = server.await {
            eprintln!("server error: {}", e);
        }
    });
    Ok(())
}

async fn proxy(client: HttpClient, req: Request<Body>, socks5_addr: String) -> hyper::Result<Response<Body>> {
    println!("req: {:?}", req);

    if Method::CONNECT == req.method() {
        if let Some(addr) = host_addr(req.uri()).await {
            tokio::task::spawn(async move {
                match req.into_body().on_upgrade().await {
                    Ok(upgraded) => {
                        if let Err(e) = tunnel(upgraded, addr, &socks5_addr).await {
                            eprintln!("server io error: {}", e);
                        };
                    }
                    Err(e) => eprintln!("upgrade error: {}", e),
                }
            });

            Ok(Response::new(Body::empty()))
        } else {
            eprintln!("CONNECT host is not socket addr: {:?}", req.uri());
            let mut resp = Response::new(Body::from("CONNECT must be to a socket address"));
            *resp.status_mut() = http::StatusCode::BAD_REQUEST;

            Ok(resp)
        }
    } else {
        client.request(req).await
    }
}

async fn host_addr(uri: &http::Uri) -> Option<SocketAddr> {
    tokio::net::lookup_host((uri.host().unwrap(), uri.port_u16().unwrap())).await.unwrap().next()
}

async fn tunnel(upgraded: Upgraded, addr: SocketAddr, socks5_addr: &str) -> std::io::Result<()> {
    let mut stream = TcpStream::connect(socks5_addr).await?;
    // let mut stream = BufStream::new(stream);
    connect(&mut stream, addr, None).await.res_convert(|_| "Connect socks5 server error".to_string())?;

    let amounts = {
        let (mut server_rd, mut server_wr) = tokio::io::split(stream);
        let (mut client_rd, mut client_wr) = tokio::io::split(upgraded);

        let client_to_server = tokio::io::copy(&mut client_rd, &mut server_wr);
        let server_to_client = tokio::io::copy(&mut server_rd, &mut client_wr);

        tokio::try_join!(client_to_server, server_to_client)
    };

    match amounts {
        Ok((from_client, from_server)) => {
            println!(
                "client wrote {} bytes and received {} bytes",
                from_client, from_server
            );
        }
        Err(e) => {
            println!("tunnel error: {}", e);
        }
    };
    Ok(())
}
