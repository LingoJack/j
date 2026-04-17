use super::bridge::WsBridge;
use std::io;
use std::net::UdpSocket;
use std::sync::Arc;
use tokio::sync::Notify;

fn detect_local_ip() -> String {
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0")
        && socket.connect("8.8.8.8:80").is_ok()
        && let Ok(addr) = socket.local_addr()
    {
        return addr.ip().to_string();
    }
    "127.0.0.1".to_string()
}

fn generate_token() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn display_qr_code(url: &str) {
    use qrcode::QrCode;
    use qrcode::render::unicode;

    println!("\n  📱 远程控制已启用\n");
    println!("  扫描下方二维码或访问:");
    println!("  \x1b[1;36m{}\x1b[0m\n", url);

    if let Ok(code) = QrCode::new(url.as_bytes()) {
        let string = code.render::<unicode::Dense1x2>().quiet_zone(true).build();
        for line in string.lines() {
            println!("  {}", line);
        }
    } else {
        println!("  ⚠️ 二维码生成失败，请手动访问上方链接");
    }

    println!("\n  等待手机连接...\n");
}

async fn bind_with_reuse(port: u16) -> io::Result<tokio::net::TcpListener> {
    let socket = tokio::net::TcpSocket::new_v4()?;
    socket.set_reuseaddr(true)?;
    #[cfg(target_os = "macos")]
    socket.set_reuseport(true)?;
    socket.bind(format!("0.0.0.0:{}", port).parse().unwrap())?;
    socket.listen(128)
}

pub fn start_remote_and_wait(port: u16) -> io::Result<(WsBridge, String)> {
    let ip = detect_local_ip();
    let token = generate_token();
    let url = format!("http://{}:{}/?token={}", ip, port, token);
    let expected_origin = format!("http://{}:{}", ip, port);

    display_qr_code(&url);

    let rt = tokio::runtime::Runtime::new()?;

    let (bridge, inbound_tx, outbound_tx) = WsBridge::new(rt);
    let client_connected = Arc::clone(&bridge.client_connected);
    let client_notify = Arc::new(Notify::new());
    let client_notify2 = Arc::clone(&client_notify);

    let listener = bridge.block_on(bind_with_reuse(port))?;

    let token_clone = token.clone();
    bridge.spawn(async move {
        super::server::run_server(
            listener,
            token_clone,
            inbound_tx,
            outbound_tx,
            client_connected,
            client_notify2,
            expected_origin,
        )
        .await;
    });

    let interrupted = bridge.block_on(async {
        tokio::select! {
            _ = client_notify.notified() => false,
            _ = tokio::signal::ctrl_c() => true,
        }
    });

    if interrupted {
        println!("\n  ⏹  已取消\n");
        return Err(io::Error::new(io::ErrorKind::Interrupted, "用户取消"));
    }

    println!("  ✅ 客户端已连接！正在启动对话界面...\n");
    std::thread::sleep(std::time::Duration::from_millis(500));

    Ok((bridge, url))
}
