pub mod http;
pub mod ui;

use std::fmt::{Display, Formatter, Write};
use crate::data::http::{HttpData, HttpMethod};
use crate::error::ProxyResult;

#[derive(Clone)]
pub enum StreamDirection {
    ClientToServer,
    ServerToClient,
}

impl Display for StreamDirection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamDirection::ClientToServer => f.write_str("ClientToServer"),
            StreamDirection::ServerToClient => f.write_str("ServerToClient"),
        }
    }
}

pub struct ProxyData {
    stream_id: String,
    direction: StreamDirection,
    buffer: [u8; 4096],
    len: usize,
}

impl ProxyData {
    pub fn new(direction: StreamDirection, buffer: [u8; 4096], len: usize, id: String) -> Self {
        Self { direction, buffer, len, stream_id: id }
    }

    pub fn direction(&self) -> &StreamDirection {
        &self.direction
    }

    pub fn buffer(&self) -> [u8; 4096] {
        self.buffer
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }
}


pub struct HttpTcpData {
    stream_id: String,
    req_raw: Vec<u8>,
    res_raw: Vec<u8>,
    reqs: Vec<HttpData>,
    ress: Vec<HttpData>,
}

impl HttpTcpData {
    fn push_req(&mut self, raw: [u8; 4096], len: usize) -> ProxyResult<()> {
        for md in HttpMethod::method_bytes() {
            if raw.starts_with(md) && self.req_raw.len() != 0 {
                let bs = self.req_raw.drain(..self.req_raw.len()).collect::<Vec<_>>();
                self.reqs.push(HttpData::from_bytes(bs, StreamDirection::ClientToServer)?);
                break;
            }
        }
        self.req_raw.extend(&raw[..len]);
        Ok(())
    }

    fn push_res(&mut self, raw: [u8; 4096], len: usize) -> ProxyResult<()> {
        if raw.starts_with(b"HTTP/1.1") && self.res_raw.len() != 0 {
            let bs = self.res_raw.drain(..self.res_raw.len()).collect::<Vec<_>>();
            self.ress.push(HttpData::from_bytes(bs, StreamDirection::ServerToClient)?);
        }
        self.res_raw.extend(&raw[..len]);
        Ok(())
    }

    pub fn push(&mut self, pd: ProxyData) -> ProxyResult<()> {
        match pd.direction {
            StreamDirection::ClientToServer => self.push_req(pd.buffer, pd.len),
            StreamDirection::ServerToClient => self.push_res(pd.buffer, pd.len)
        }
    }

    pub fn new() -> Self {
        Self {
            stream_id: "".to_string(),
            req_raw: vec![],
            res_raw: vec![],
            reqs: vec![],
            ress: vec![],
        }
    }
}

#[derive(Eq, PartialEq)]
pub enum FilterMode {
    None,
    XHR,
    Document,
    Css,
    Js,
    Font,
    Image,
    Media,
    Ws,
}

impl FilterMode {
    pub fn modes() -> [FilterMode; 9] {
        [FilterMode::None, FilterMode::XHR, FilterMode::Document, FilterMode::Css, FilterMode::Js,
            FilterMode::Font, FilterMode::Image, FilterMode::Media, FilterMode::Ws]
    }
}

impl Display for FilterMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FilterMode::None => f.write_str("无"),
            FilterMode::XHR => f.write_str("XHR"),
            FilterMode::Document => f.write_str("文档"),
            FilterMode::Css => f.write_str("Css"),
            FilterMode::Js => f.write_str("Js"),
            FilterMode::Font => f.write_str("字体"),
            FilterMode::Image => f.write_str("图片"),
            FilterMode::Media => f.write_str("媒体"),
            FilterMode::Ws => f.write_str("套接字")
        }
    }
}