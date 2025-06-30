use crate::data::http::body::HttpBody;
use crate::data::http::header::HttpHeader;
use crate::data::StreamDirection;
use crate::error::ProxyResult;

mod header;
mod body;

const HTTP_HEAD_BODY_GAP: &'static [u8] = b"\r\n\r\n";

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
    PartialContent = 206,
    NotModified = 304,

}

impl HttpStatus {
    pub fn from_stream_raw(code: &str) -> ProxyResult<HttpStatus> {
        let code = code.parse::<i32>()?;
        match code {
            200 => Ok(HttpStatus::OK),
            304 => Ok(HttpStatus::NotModified),
            _ => Err("未知的Http状态码".into())
        }
    }
}

pub struct HttpData {
    header: HttpHeader,
    body: HttpBody,
}

impl HttpData {
    pub fn from_bytes(mut bs: Vec<u8>, direction: StreamDirection) -> ProxyResult<HttpData> {
        let pos = bs.windows(HTTP_HEAD_BODY_GAP.len()).position(|w| w == HTTP_HEAD_BODY_GAP).ok_or("HTTP数据错误")?;
        let hbs = bs.drain(..pos).collect::<Vec<_>>();
        let hdr = match direction {
            StreamDirection::ClientToServer => HttpHeader::from_client(hbs)?,
            StreamDirection::ServerToClient => HttpHeader::from_server(hbs)?,
        };
        let body = HttpBody::from_bytes(bs.drain(HTTP_HEAD_BODY_GAP.len()..).collect());
        Ok(HttpData {
            header: hdr,
            body,
        })
    }
}


pub enum HttpMethod {
    GET,
    POST,
    PUT,
    HEAD,
    CONNECT,
    TRACE,
    PATCH,
    OPTIONS,
    DELETE,
}

impl HttpMethod {
    pub fn method_bytes() -> Vec<&'static [u8]> {
        vec![b"GET", b"POST", b"PUT", b"HEAD", b"CONNECT", b"TRACE", b"PATCH", b"OPTIONS", b"DELETE"]
    }
}