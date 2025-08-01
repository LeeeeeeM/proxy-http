use std::fmt::{Display, Formatter};

#[derive(Eq, PartialEq, Clone)]
pub enum ProxyTab {
    Header,
    Param,
    PreView,
    Cookie,
    ReqRaw,
    RespRaw,
}
impl ProxyTab {
    pub fn tabs() -> Vec<ProxyTab> {
        vec![ProxyTab::Header, ProxyTab::Param, ProxyTab::PreView,  ProxyTab::Cookie, ProxyTab::ReqRaw, ProxyTab::RespRaw]
    }
}

impl Display for ProxyTab {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ProxyTab::Header => f.write_str("标头"),
            ProxyTab::PreView => f.write_str("预览"),
            ProxyTab::Param => f.write_str("负载"),
            ProxyTab::Cookie => f.write_str("Cookie"),
            ProxyTab::ReqRaw => f.write_str("原始请求"),
            ProxyTab::RespRaw => f.write_str("原始响应"),
        }
    }
}