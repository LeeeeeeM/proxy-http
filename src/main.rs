mod cert;

use std::error::Error;
use std::io::BufReader;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs};
use std::ptr::read;
use std::sync::Arc;
use rustls::{ClientConfig, RootCertStore};
use rustls::server::ClientCertVerifierBuilder;
use rustls_pemfile::Item;
use rustls_pki_types::{DnsName, PrivateKeyDer, ServerName};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::rustls::{ServerConfig, SupportedProtocolVersion};
use tokio_rustls::{TlsAcceptor, TlsConnector};

#[tokio::main]
async fn main() {
    // println!("{:?}"," ".as_bytes().to_vec());
    start_server().await.unwrap();
    // start_socks5_server().await.unwrap()
}


async fn start_socks5_server() -> Result<(), Box<dyn Error>> {
    let listen = TcpListener::bind("127.0.0.1:7091").await?;
    loop {
        //接受一个新连接
        let (stream, addr) = listen.accept().await?;
        println!("{}", addr);
        //启动一个线程，避免造成其他连接阻塞，影响网络体验
        tokio::spawn(async move {
            handle_socks5_client(stream).await.unwrap(); //Broken pipe这个是异常断开，就是我们的浏览器，突然关闭窗口了
        });
    }
}

async fn handle_socks5_client(mut inbound: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut buffer = [0; 1024];
    let len = inbound.read(&mut buffer).await?;
    //这里还是和之前的一样，先看一下数据
    println!("{:?}", buffer[..len].to_vec());
    //我们这里给客户端返回不认证
    inbound.write(&[5, 0]).await?;
    inbound.flush().await?;
    //因为我们选择了不认证，所以这里直接跳过认证环节，客户端会发送请求数据
    let mut buffer = [0; 1024];
    let len = inbound.read(&mut buffer).await?;
    //这里先看一下客户端发过的来数据
    println!("{:?}", buffer[..len].to_vec());
    //现在来解析地址
    let addr = match buffer[3] {
        //IPv4
        0x01 => {
            let host = u32::from_be_bytes(buffer[4..8].try_into().unwrap());
            let port = u16::from_be_bytes(buffer[8..10].try_into().unwrap());
            let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from(host), port));
            addr
        }
        //域名，第一个字节为长度
        0x03 => {
            let len = buffer[4] as usize;
            let domain = String::from_utf8(buffer[5..len + 5].to_vec())?;
            let port = u16::from_be_bytes(buffer[len + 5..len + 7].try_into().unwrap());
            //这里做了一个域名解析，我们以IPv4为例，暂不支持IPv6
            let addr = format!("{}:{}", domain, port).to_socket_addrs()?.find(|x| x.is_ipv4()).ok_or("解析IPv4地址失败")?;
            addr
        }
        _ => {
            //这里我们做一个不支持的地址类型
            inbound.write(&[5, 8, 0, 1, 127, 0, 0, 1, 0, 80]).await?; //这里的127,0,0,1,0,80为地址127.0.0.1:80无意义
            inbound.shutdown().await?;
            return Err("地址类型不支持".into());
        }
    };
    println!("{}", addr);
    //建立连接，并返回
    let outbound = match TcpStream::connect(addr).await {
        Ok(res) => res,
        Err(e) => {
            //连接目标地址失败
            inbound.write(&[5, 5, 0, 1, 127, 0, 0, 1, 0, 80]).await?;
            inbound.shutdown().await?;
            return Err(e.into());
        }
    };
    inbound.write(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 80]).await?; //这里的127,0,0,1,0,80为地址127.0.0.1:80无意义
    //这里我们就完成了socks5代理的建立
    copy_io(inbound, outbound).await?;
    Ok(())
}


//我们先在本地启动一个服务，监听7090端口
async fn start_server() -> Result<(), Box<dyn Error>> {
    let listen = TcpListener::bind("0.0.0.0:7090").await?;
    loop {
        //接受一个新连接
        let (stream, addr) = listen.accept().await?;
        println!("{}", addr);
        //启动一个线程，避免造成其他连接阻塞，影响网络体验
        tokio::spawn(async move {
            handle_client_stream(stream).await.unwrap();
        });
        //因为这里对于刚接触HTTP协议的数据的人来说并不知道其内容是怎样的，可以先读一个数据看看
        // let mut buffer = [0; 4096];
        // let len = stream.read(&mut buffer).await?;
        // println!("{}", String::from_utf8_lossy(&buffer[..len]));
        // GET http://www.baidu.com/ HTTP/1.1 真实的请求是没有http://www.baidu.com
        // Host: www.baidu.com
        // User-Agent: curl/7.76.1
        // Accept: */*
        // Proxy-Connection: Keep-Alive
        //需要从请求体中读出真实的服务器地址，然后建立连接

    }
}

async fn handle_client_stream(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut buffer = [0; 4096];
    let len = stream.read(&mut buffer).await?;

    if buffer.starts_with(b"CONNECT") {
        handle_https(stream, buffer, len).await?;
    } else {
        // handle_http(stream, buffer, len).await?;
    }
    Ok(())
}

async fn handle_https(mut stream: TcpStream, buffer: [u8; 4096], len: usize) -> Result<(), Box<dyn Error>> {
    let info = String::from_utf8(buffer[..len].to_vec())?;
    let addr = regex_find("CONNECT (.*?) ", info.as_str())?;
    if addr.len() == 0 { return Err("获取HTTPS真实地址失败".into()); }
    stream.write(b"HTTP/1.1 200 OK\r\n\r\n").await?;
    stream.flush().await?;
    println!("{}", addr[0]);
    //从这里开始，两个stream之间交互的就是真实的https数据了
    let sni = addr[0].split(":").next().unwrap();
    let acceptor = gen_acceptor_for_sni(sni)?;
    let inbound = acceptor.accept(stream).await?;
    let mut root_ca = RootCertStore::empty();
    root_ca.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let client_config = ClientConfig::builder().with_root_certificates(root_ca).with_no_client_auth();
    let outbound = TcpStream::connect(&addr[0]).await?;
    let connector = TlsConnector::from(Arc::new(client_config));
    println!("{}", sni);
    let server_name = ServerName::DnsName(DnsName::try_from(sni.to_string())?);
    let outbound = connector.connect(server_name, outbound).await?;
    // //这里我们就实现了HTTPS解密，但是我们的根证书还没安装
    // //sudo cp sca.pem /etc/pki/ca-trust/source/anchors/
    // //sudo update-ca-trust
    copy_io(inbound, outbound).await
}

async fn handle_http(stream: TcpStream, buffer: [u8; 4096], len: usize) -> Result<(), Box<dyn Error>> {
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
    copy_io(stream, outbound).await
}

async fn copy_io<I, O>(inbound: I, outbound: O) -> Result<(), Box<dyn Error>>
where
    I: AsyncReadExt + AsyncWriteExt + Send + Unpin + 'static,
    O: AsyncReadExt + AsyncWriteExt + Send + Unpin + 'static,
{
    let (mut inbound_reader, mut inbound_writer) = tokio::io::split(inbound);
    let (mut outbound_reader, mut outbound_writer) = tokio::io::split(outbound);
    let rt1 = tokio::spawn(async move {
        let _ = tokio::io::copy(&mut inbound_reader, &mut outbound_writer).await;
    });
    let rt2 = tokio::spawn(async move {
        let _ = tokio::io::copy(&mut outbound_reader, &mut inbound_writer).await;
    });
    tokio::join!(rt1,rt2);
    Ok(())
}

fn regex_find(rex: &str, context: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let regx = regex::RegexBuilder::new(rex).build()?;
    let mut res = vec![];
    for re in regx.captures_iter(context) {
        let mut r = vec![];
        for index in 0..re.len() {
            r.push(re[index].to_string());
        }
        if r.len() > 1 { r.remove(0); }
        res.extend(r);
    };
    Ok(res)
}


//这里需要实现一个TlsAcceptor才能解密
fn gen_acceptor_for_sni(sni: impl AsRef<str>) -> Result<TlsAcceptor, Box<dyn Error>> {
    //这里先要生成证书
    let (pem, key) = cert::gen_cert_for_sni(sni.as_ref(), "sca.pem", "sca.key")?;
    let ca_bs = pem.into_bytes();
    let key_bs = key.into_bytes();
    let mut reader = BufReader::new(ca_bs.as_slice());
    let item = rustls_pemfile::read_one(&mut reader).transpose().ok_or("读取证书失败")??;
    let sni_cert = match item {
        Item::X509Certificate(cert) => cert,
        _ => return Err("不支持的证书".into()),
    };
    let mut reader = BufReader::new(key_bs.as_slice());
    let item = rustls_pemfile::read_one(&mut reader).transpose().ok_or("读取证书密钥失败")??;
    let sni_key = match item {
        Item::Pkcs1Key(key) => PrivateKeyDer::Pkcs1(key),
        Item::Pkcs8Key(key) => PrivateKeyDer::Pkcs8(key),
        Item::Sec1Key(key) => PrivateKeyDer::Sec1(key),
        _ => return Err("不支持的证书密钥类型".into()),
    };
    let config = ServerConfig::builder_with_protocol_versions(&rustls::ALL_VERSIONS)
        .with_no_client_auth().with_single_cert(vec![sni_cert], sni_key)?;
    let acceptor = TlsAcceptor::from(Arc::new(config));
    Ok(acceptor)
}