use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use crate::error::ProxyResult;

async fn start_socks5_server() -> ProxyResult<()> {
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

async fn handle_socks5_client(mut inbound: TcpStream) -> ProxyResult<()> {
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