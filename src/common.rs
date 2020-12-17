use std::ffi::CStr;
use std::os::raw::c_char;

use tokio::io::{Error, ErrorKind, Result};

pub trait OptionConvert<T> {
    fn option_to_res(self, msg: &str) -> Result<T>;
}

impl<T> OptionConvert<T> for Option<T> {
    fn option_to_res(self, msg: &str) -> Result<T> {
        option_convert(self, msg)
    }
}

pub trait StdResConvert<T, E> {
    fn res_convert(self, f: fn(E) -> String) -> Result<T>;
}

impl<T, E> StdResConvert<T, E> for std::result::Result<T, E> {
    fn res_convert(self, f: fn(E) -> String) -> Result<T> {
        std_res_convert(self, f)
    }
}

pub trait StdResAutoConvert<T, E: ToString> {
    fn res_auto_convert(self) -> Result<T>;
}

impl<T, E: ToString> StdResAutoConvert<T, E> for std::result::Result<T, E> {
    fn res_auto_convert(self) -> Result<T> {
        std_res_convert(self, |e| e.to_string())
    }
}

fn option_convert<T>(o: Option<T>, msg: &str) -> Result<T> {
    match o {
        Some(v) => Ok(v),
        None => Err(Error::new(ErrorKind::Other, msg))
    }
}

fn std_res_convert<T, E>(res: std::result::Result<T, E>, f: fn(E) -> String) -> Result<T> {
    match res {
        Ok(v) => Ok(v),
        Err(e) => Err(Error::new(ErrorKind::Other, f(e)))
    }
}

pub fn str_convert(str: *const c_char, len: usize) -> Result<String> {
    let str = unsafe { CStr::from_ptr(str) };
    let str = str.to_str().res_auto_convert()?;
    Ok(String::from(&str[..len]))
}