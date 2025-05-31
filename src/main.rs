use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    // println!("{:?}"," ".as_bytes().to_vec());
    start_server().await.unwrap();
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
        handle_http(stream, buffer, len).await?;
    }
    Ok(())
}

async fn handle_https(mut stream: TcpStream, buffer: [u8; 4096], len: usize) -> Result<(), Box<dyn Error>> {
    let info = String::from_utf8(buffer[..len].to_vec())?;
    let addr = regex_find("CONNECT (.*?) ", info.as_str())?;
    if addr.len() == 0 { return Err("获取HTTPS真实地址失败".into()); }
    stream.write(b"HTTP/1.1 200 OK\r\n\r\n").await?;
    stream.flush().await?;
    //从这里开始，两个stream之间交互的就是真实的https数据了
    let outbound = TcpStream::connect(&addr[0]).await?;
    let (mut inbound_reader, mut inbound_writer) = tokio::io::split(stream);
    let (mut outbound_reader, mut outbound_writer) = tokio::io::split(outbound);
    let rt1 = tokio::spawn(async move {
        let _ = tokio::io::copy(&mut inbound_reader, &mut outbound_writer).await;
    });
    let rt2 = tokio::spawn(async move {
        let _ = tokio::io::copy(&mut outbound_reader, &mut inbound_writer).await; //这里报错是因为连接异常中断，这个我们的代理不用管
    });
    tokio::join!(rt1,rt2);
    Ok(())
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
    let (mut inbound_reader, mut inbound_writer) = tokio::io::split(stream);
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