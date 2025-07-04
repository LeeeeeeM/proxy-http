use std::collections::HashMap;
use crate::data::http::{HttpStatus, HttpVersion};
use crate::error::ProxyResult;

pub struct HttpHeader {
    method: String,
    uri: String,
    version: HttpVersion,
    status: HttpStatus,
    keys: HashMap<String, String>,
}

impl HttpHeader {
    pub fn new() -> HttpHeader {
        HttpHeader {
            method: "".to_string(),
            uri: "".to_string(),
            version: HttpVersion::Http10,
            status: HttpStatus::OK,
            keys: Default::default(),
        }
    }
    pub fn from_client(raw: Vec<u8>) -> ProxyResult<HttpHeader> {
        let mut http_header = HttpHeader::new();
        let header_string = String::from_utf8(raw.to_vec())?.replace("\r\n", "\n");
        println!("{}",header_string);
        let line = header_string.lines().next().ok_or("传入的数据错误")?;
        let mut items = line.split(" ");
        //这里解析请求头的第一行
        http_header.method = items.next().ok_or("获取method失败")?.to_string();
        http_header.uri = items.next().ok_or("获取uri失败")?.to_string();
        http_header.version = HttpVersion::from_stream_raw(items.next().ok_or("获取version失败")?)?;
        http_header.handle_key_value(header_string)?;
        //到这里，客户端的请求头就解析完成了
        Ok(http_header)
    }

    pub fn from_server(raw: Vec<u8>) -> ProxyResult<HttpHeader> {
        let mut http_header = HttpHeader::new();
        let header_string = String::from_utf8(raw.to_vec())?.replace("\r\n", "\n");
        println!("{}",header_string);
        let line = header_string.lines().next().ok_or("传入的数据错误")?;
        let mut items = line.split(" ");
        http_header.version = HttpVersion::from_stream_raw(items.next().ok_or("获取version失败")?)?;
        http_header.status = HttpStatus::from_stream_raw(items.next().ok_or("获取code失败")?)?;
        http_header.handle_key_value(header_string)?;
        Ok(http_header)
        //到这里，服务器的响应头就解析完成了
    }

    fn handle_key_value(&mut self, hdr_str: String) -> ProxyResult<()> {
        for (i, line) in hdr_str.split("\n").enumerate() {
            if i == 0 { continue; }
            let mut items = line.split(": ");
            let key = items.next().ok_or("解析请求头字段失败")?;
            let value = items.collect::<Vec<_>>().join(": "); //这里呢是防止有些字段是空的和中间有": "。
            self.keys.insert(key.to_string(), value.to_string());
        }
        Ok(())
    }

    pub fn keys(&self)->&HashMap<String, String>{
        &self.keys
    }
}