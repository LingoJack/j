//! 远程控制模块入口：启动服务器 + 等待客户端

pub mod bridge;
pub mod protocol;
pub mod server;

use bridge::WsBridge;
use std::io;
use std::net::UdpSocket;
use std::sync::Arc;
use tokio::sync::Notify;

/// 检测本机局域网 IP
fn detect_local_ip() -> String {
    // 通过 UDP 连接来检测本机出口 IP
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0")
        && socket.connect("8.8.8.8:80").is_ok()
        && let Ok(addr) = socket.local_addr()
    {
        return addr.ip().to_string();
    }
    "127.0.0.1".to_string()
}

/// 生成 6 位随机 token
fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "abcdefghijkmnpqrstuvwxyz23456789".chars().collect();
    (0..6)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

/// 在终端显示二维码
fn display_qr_code(url: &str) {
    use qrcode::QrCode;

    println!("\n  📱 远程控制已启用\n");
    println!("  扫描下方二维码或访问:");
    println!("  \x1b[1;36m{}\x1b[0m\n", url);

    if let Ok(code) = QrCode::new(url.as_bytes()) {
        let string = code
            .render::<char>()
            .quiet_zone(true)
            .module_dimensions(2, 1)
            .build();
        for line in string.lines() {
            println!("  {}", line);
        }
    } else {
        println!("  ⚠️ 二维码生成失败，请手动访问上方链接");
    }

    println!("\n  等待手机连接...\n");
}

/// 创建带 SO_REUSEADDR 的 TcpListener，避免 "Address already in use" 错误
async fn bind_with_reuse(port: u16) -> io::Result<tokio::net::TcpListener> {
    let socket = tokio::net::TcpSocket::new_v4()?;
    socket.set_reuseaddr(true)?;
    #[cfg(target_os = "macos")]
    socket.set_reuseport(true)?;
    socket.bind(format!("0.0.0.0:{}", port).parse().unwrap())?;
    socket.listen(128)
}

/// 启动远程控制服务器并等待客户端连接
///
/// 返回 `(WsBridge, url)` 或 IO 错误。
/// 等待期间支持 Ctrl+C 中断。
pub fn start_remote_and_wait(port: u16) -> io::Result<(WsBridge, String)> {
    let ip = detect_local_ip();
    let token = generate_token();
    let url = format!("http://{}:{}/?token={}", ip, port, token);

    // 显示二维码
    display_qr_code(&url);

    // 创建 tokio runtime
    let rt = tokio::runtime::Runtime::new()?;

    // 创建 bridge，把 runtime 所有权交给 bridge
    let (bridge, inbound_tx, outbound_tx) = WsBridge::new(rt);
    let client_connected = Arc::clone(&bridge.client_connected);
    let client_notify = Arc::new(Notify::new());
    let client_notify2 = Arc::clone(&client_notify);

    // 通过 bridge 内部持有的 runtime 来启动服务器和等待连接
    let listener = bridge.block_on(bind_with_reuse(port))?;

    // 在后台 task 中运行服务器
    let token_clone = token.clone();
    bridge.spawn(async move {
        server::run_server(
            listener,
            token_clone,
            inbound_tx,
            outbound_tx,
            client_connected,
            client_notify2,
        )
        .await;
    });

    // 等待客户端连接，同时监听 Ctrl+C
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
