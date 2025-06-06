use std::io::ErrorKind::Other;
use std::pin::Pin;
use std::task::{Context, Poll};

pub type ProxyResult<T> = Result<T, ProxyError>;

#[derive(Debug)]
pub struct ProxyError {
    msg: String,
}

impl ProxyError {
    pub fn to_string(&self) -> String { self.msg.clone() }
}

impl<E: ToString> From<E> for ProxyError {
    fn from(e: E) -> Self {
        ProxyError { msg: e.to_string() }
    }
}
//这几个impl是为了把我们的错误类型转化为其他，使用？可以把我们的错误类传转成其他
impl From<ProxyError> for Box<dyn std::error::Error> {
    fn from(value: ProxyError) -> Self {
        Box::new(std::io::Error::new(Other, value.to_string()))
    }
}

impl From<ProxyError> for Box<dyn std::error::Error + Sync> {
    fn from(value: ProxyError) -> Self {
        Box::new(std::io::Error::new(Other, value.to_string()))
    }
}

impl From<ProxyError> for Box<dyn std::error::Error + Sync + Unpin> {
    fn from(value: ProxyError) -> Self {
        Box::new(std::io::Error::new(Other, value.to_string()))
    }
}

//这里主要是为了我们定义的错误类型可以在异步中传递
impl Future for ProxyError {
    type Output = String;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.msg.clone())
    }
}

unsafe impl Send for ProxyError {}