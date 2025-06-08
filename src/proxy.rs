use std::sync::Arc;
use log::{error, trace};
use rustls::{ClientConfig, RootCertStore};
use rustls_pki_types::{DnsName, ServerName};
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync;
use tokio::task::{JoinError, JoinHandle};
use tokio_rustls::TlsConnector;
use crate::error::{ProxyError, ProxyResult};
use crate::{gen_acceptor_for_sni, regex_find};
use crate::data::{ProxyData, StreamDirection};

//
pub struct ProxyStream {
    inbound: TcpStream,
    sender: sync::mpsc::Sender<ProxyData>,
}

impl ProxyStream {
    pub fn new(inbound: TcpStream, sender: sync::mpsc::Sender<ProxyData>) -> ProxyStream {
        ProxyStream { inbound, sender }
    }

    async fn copy<'a, I, O>(mut reader: ReadHalf<I>, mut writer: WriteHalf<O>, direction: StreamDirection, sender: sync::mpsc::Sender<ProxyData>) -> JoinHandle<ProxyResult<()>>
    where
        I: AsyncReadExt + Send + Unpin + 'static,
        O: AsyncWriteExt + Send + Unpin + 'static,
    {
        tokio::spawn(async move {
            loop {
                let mut buffer = [0; 4096];
                let len = reader.read(&mut buffer).await?;
                //及时把数据发送出去，减少延时
                writer.write(&buffer[..len]).await?;
                if len == 0 { break; } //读取长度为0时，此tcp连接已断开
                let data = ProxyData::new(direction.clone(), buffer, len);
                sender.send(data).await?;
            }
            Ok::<(), ProxyError>(())
        })
    }

    async fn copy_io<I, O>(inbound: I, outbound: O, sender: sync::mpsc::Sender<ProxyData>) -> ProxyResult<()>
    where
        I: AsyncReadExt + AsyncWriteExt + Send + Unpin + 'static,
        O: AsyncReadExt + AsyncWriteExt + Send + Unpin + 'static,
    {
        let res_func = |res: Result<ProxyResult<()>, JoinError>, direction: StreamDirection| {
            match res {
                Ok(r) => match r {
                    Ok(()) => {}
                    Err(e) => error!("{}{}",direction,e.to_string())
                }
                Err(e) => error!("{}{}",direction,e.to_string())
            }
        };
        let (inbound_reader, inbound_writer) = tokio::io::split(inbound);
        let (outbound_reader, outbound_writer) = tokio::io::split(outbound);
        let rt1 = ProxyStream::copy(inbound_reader, outbound_writer, StreamDirection::ClientToServer, sender.clone()).await;
        let rt2 = ProxyStream::copy(outbound_reader, inbound_writer, StreamDirection::ServerToClient, sender).await;
        let (r1, r2) = tokio::join!(rt1,rt2);
        res_func(r1, StreamDirection::ClientToServer);
        res_func(r2, StreamDirection::ServerToClient);
        Ok(())
    }

    async fn handle_http(self, buffer: [u8; 4096], len: usize) -> ProxyResult<()> {
        let http_prefix = b"http://";
        let start_pos = buffer.windows(http_prefix.len()).position(|b| b == http_prefix).ok_or("获取HTTP地址失败")?;
        let end_pos = buffer[start_pos + http_prefix.len()..len].iter().position(|b| *b == b'/').ok_or("获取HTTP地址失败")? + start_pos + http_prefix.len();
        println!("{} {}", start_pos, end_pos);
        //获取真实服务器地址，端口为80的会自动省略
        let addr = String::from_utf8(buffer[start_pos + http_prefix.len()..end_pos].to_vec())?;
        println!("{}", addr);
        let host = addr.split(":").next().unwrap();
        let port = match addr.contains(":") {
            true => addr.split(":").last().ok_or("获取端口失败")?.parse::<u16>()?,
            false => 80
        };
        // 这里我们就拿到了真实的服务器地址
        println!("{}:{}", host, port);
        //与真实服务器建立连接，并把两个stream相互复制
        let mut outbound = TcpStream::connect(format!("{}:{}", host, port)).await?;
        // outbound.write(&buffer[..len]).await?;
        outbound.write(&buffer[..start_pos]).await?;
        outbound.write(&buffer[end_pos..len]).await?;
        ProxyStream::copy_io(self.inbound, outbound, self.sender).await
    }

    async fn handle_https(mut self, buffer: [u8; 4096], len: usize) -> ProxyResult<()> {
        let info = String::from_utf8(buffer[..len].to_vec())?;
        let addr = regex_find("CONNECT (.*?) ", info.as_str())?;
        if addr.len() == 0 { return Err("获取HTTPS真实地址失败".into()); }
        self.inbound.write(b"HTTP/1.1 200 OK\r\n\r\n").await?;
        self.inbound.flush().await?;
        //从这里开始，两个stream之间交互的就是真实的https数据了
        let sni = addr[0].split(":").next().unwrap();
        trace!("已解析到https地址：{}；SNI：{}",addr[0],sni);
        let acceptor = gen_acceptor_for_sni(sni)?;
        let inbound = acceptor.accept(self.inbound).await?;
        let mut root_ca = RootCertStore::empty();
        root_ca.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let client_config = ClientConfig::builder().with_root_certificates(root_ca).with_no_client_auth();
        let outbound = TcpStream::connect(&addr[0]).await?;
        let connector = TlsConnector::from(Arc::new(client_config));
        let server_name = ServerName::DnsName(DnsName::try_from(sni.to_string())?);
        let outbound = connector.connect(server_name, outbound).await?;
        // //这里我们就实现了HTTPS解密，但是我们的根证书还没安装
        // //sudo cp sca.pem /etc/pki/ca-trust/source/anchors/
        // //sudo update-ca-trust
        ProxyStream::copy_io(inbound, outbound, self.sender).await
    }

    pub async fn start(mut self) -> ProxyResult<()> {
        let mut buffer = [0; 4096];
        let len = self.inbound.read(&mut buffer).await?;

        if buffer.starts_with(b"CONNECT") {
            self.handle_https(buffer, len).await?;
        } else {
            self.handle_http(buffer, len).await?;
        }
        Ok(())
    }
}