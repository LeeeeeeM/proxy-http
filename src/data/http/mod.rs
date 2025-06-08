use std::io::Split;
use crate::data::http::body::HttpBody;
use crate::data::http::header::HttpHeader;
use crate::error::ProxyResult;

mod header;
mod body;

//现在我们来解析一下HTTP数据

pub enum HttpVersion {
    Http10,
    Http11,
    Http20,
    Http30,
}

impl HttpVersion {
    pub fn from_stream_raw(ver: &str) -> ProxyResult<HttpVersion> {
        match ver {
            "HTTP/1.0" => Ok(HttpVersion::Http10),
            "HTTP/1.1" => Ok(HttpVersion::Http11),
            "HTTP/2.0" => Ok(HttpVersion::Http20),
            "HTTP/3.0" => Ok(HttpVersion::Http30),
            &_ => Err("请求版本解析失败".into())
        }
    }
}

pub enum HttpStatus {
    OK = 200,

}

impl HttpStatus {
    pub fn from_stream_raw(code: &str) -> ProxyResult<HttpStatus> {
        let code = code.parse::<i32>()?;
        match code {
            200 => Ok(HttpStatus::OK),
            _ => Err("未知的Http状态码".into())
        }
    }
}

pub struct HttpData {
    header: HttpHeader,
    body: HttpBody,
}