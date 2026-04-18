//! HTTP + WebSocket 混合服务器（ECDH P-256 加密通信）

use super::crypto;
use super::protocol::{WsInbound, WsOutbound};
use crate::assets::Assets;
use futures::SinkExt;
use futures::stream::StreamExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, broadcast, mpsc, watch};
use tokio_tungstenite::tungstenite::protocol::Message;

/// 服务端 ping 间隔（秒）
const PING_INTERVAL_SECS: u64 = 15;
/// 未收到 pong 的超时时间（秒）
const PONG_TIMEOUT_SECS: u64 = 30;
/// ECDH 密钥协商超时（秒）
const KEY_EXCHANGE_TIMEOUT_SECS: u64 = 10;

/// WebSocket 连接共享状态（通道 + 连接标志 + kick 信号）
struct WsConnectionState {
    inbound_tx: mpsc::Sender<WsInbound>,
    outbound_tx: broadcast::Sender<WsOutbound>,
    client_connected: Arc<AtomicBool>,
    client_notify: Arc<Notify>,
    kick_tx: watch::Sender<u64>,
    kick_rx: watch::Receiver<u64>,
}

/// 启动 HTTP + WS 服务器
///
/// - `GET /` → 返回嵌入的 remote.html
/// - `GET /ws?token=xxx` → WebSocket 升级（含 Origin 校验 + ECDH 协商）
pub async fn run_server(
    listener: TcpListener,
    token: String,
    inbound_tx: mpsc::Sender<WsInbound>,
    outbound_tx: broadcast::Sender<WsOutbound>,
    client_connected: Arc<AtomicBool>,
    client_notify: Arc<Notify>,
    expected_origin: String,
) {
    // kick_tx 用于踢掉旧的 WS 连接：每次发送新值，旧连接检测到变化后退出
    let (kick_tx, kick_rx) = watch::channel(0u64);

    loop {
        let Ok((stream, _addr)) = listener.accept().await else {
            continue;
        };

        let token = token.clone();
        let expected_origin = expected_origin.clone();
        let ws_state = WsConnectionState {
            inbound_tx: inbound_tx.clone(),
            outbound_tx: outbound_tx.clone(),
            client_connected: Arc::clone(&client_connected),
            client_notify: Arc::clone(&client_notify),
            kick_tx: kick_tx.clone(),
            kick_rx: kick_rx.clone(),
        };

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, &token, ws_state, &expected_origin).await {
                crate::util::log::write_error_log(
                    "[remote::server]",
                    &format!("连接处理错误: {}", e),
                );
            }
        });
    }
}

/// 从 HTTP 请求头中提取指定 header 的值
fn extract_header<'a>(request: &'a str, header_name: &str) -> Option<&'a str> {
    let lower_name = header_name.to_ascii_lowercase();
    for line in request.lines().skip(1) {
        // header 结束
        if line.is_empty() || line == "\r" {
            break;
        }
        if let Some((key, value)) = line.split_once(':')
            && key.trim().to_ascii_lowercase() == lower_name
        {
            return Some(value.trim().trim_end_matches('\r'));
        }
    }
    None
}

async fn handle_connection(
    stream: TcpStream,
    token: &str,
    ws_state: WsConnectionState,
    expected_origin: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 先 peek 请求头判断是 HTTP 还是 WS 升级
    let mut buf = [0u8; 4096];
    let n = stream.peek(&mut buf).await?;
    let request_str = String::from_utf8_lossy(&buf[..n]);

    // 解析请求行
    let first_line = request_str.lines().next().unwrap_or("");

    if first_line.starts_with("GET / ") || first_line.starts_with("GET /?") {
        // 提取查询参数中的 token
        let query_token = extract_query_param(&request_str, "token");
        if query_token.as_deref() != Some(token) {
            // 无 token 的 / 请求也返回 HTML（token 在 WS 连接时校验）
        }
        serve_html(stream).await?;
        return Ok(());
    }

    if first_line.contains("/ws") {
        // 验证 token
        let query_token = extract_query_param(&request_str, "token");
        if query_token.as_deref() != Some(token) {
            serve_error(stream, 403, "Forbidden: invalid token").await?;
            return Ok(());
        }

        // Origin 校验：如果存在 Origin 头则必须匹配，不存在则放行（非浏览器客户端）
        if let Some(origin) = extract_header(&request_str, "Origin")
            && origin != expected_origin
        {
            crate::util::log::write_error_log(
                "[remote::server]",
                &format!("Origin 校验失败: {} != {}", origin, expected_origin),
            );
            serve_error(stream, 403, "Forbidden: origin mismatch").await?;
            return Ok(());
        }

        // 踢掉旧连接：发送新的 kick 信号，旧的 handle_websocket 会检测到并退出
        let new_ver = {
            let cur = *ws_state.kick_tx.borrow();
            cur.wrapping_add(1)
        };
        let _ = ws_state.kick_tx.send(new_ver);

        // 等待旧连接释放（最多 2 秒）
        for _ in 0..20 {
            if !ws_state.client_connected.load(Ordering::Relaxed) {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // WebSocket 升级
        let ws_stream = tokio_tungstenite::accept_async(stream).await?;
        ws_state.client_connected.store(true, Ordering::Relaxed);
        ws_state.client_notify.notify_one();

        handle_websocket(
            ws_stream,
            ws_state.inbound_tx,
            ws_state.outbound_tx,
            &ws_state.client_connected,
            ws_state.kick_rx,
        )
        .await;

        ws_state.client_connected.store(false, Ordering::Relaxed);
        return Ok(());
    }

    // 未知路径
    serve_error(stream, 404, "Not Found").await?;
    Ok(())
}

/// ECDH 密钥协商：返回派生的 AES-256 密钥，或 None 表示失败/超时
async fn perform_key_exchange(
    ws_tx: &mut futures::stream::SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, Message>,
    ws_rx: &mut futures::stream::SplitStream<tokio_tungstenite::WebSocketStream<TcpStream>>,
) -> Option<[u8; 32]> {
    let log = |msg: &str| {
        crate::util::log::write_info_log("[remote::key_exchange]", msg);
    };

    // 1. 生成服务端密钥对
    let (server_sk, server_pk) = crypto::generate_keypair();
    let server_pk_b64 = crypto::export_public_key(&server_pk);
    log(&format!("server_pk 长度: {}", server_pk_b64.len()));

    // 2. 发送 server_hello（明文 JSON）
    let hello = WsOutbound::ServerHello {
        server_pk: server_pk_b64,
    };
    let hello_json = serde_json::to_string(&hello).ok()?;
    if ws_tx.send(Message::Text(hello_json.into())).await.is_err() {
        log("发送 server_hello 失败");
        return None;
    }
    log("已发送 server_hello");

    // 3. 等待客户端 key_exchange 消息（超时 10 秒）
    let timeout = tokio::time::Duration::from_secs(KEY_EXCHANGE_TIMEOUT_SECS);
    let client_pk_b64 = match tokio::time::timeout(timeout, async {
        while let Some(result) = ws_rx.next().await {
            match result {
                Ok(Message::Text(text)) => {
                    log(&format!("收到 Text 消息: {}", &text[..text.len().min(200)]));
                    if let Ok(WsInbound::KeyExchange { client_pk }) =
                        serde_json::from_str::<WsInbound>(&text)
                    {
                        return Some(client_pk);
                    }
                }
                Ok(Message::Close(frame)) => {
                    log(&format!("客户端关闭连接: {:?}", frame));
                    return None;
                }
                Ok(other) => {
                    log(&format!("收到非 Text 消息: {:?}", other));
                }
                Err(e) => {
                    log(&format!("ws_rx 错误: {}", e));
                    return None;
                }
            }
        }
        log("ws_rx 流结束");
        None
    })
    .await
    {
        Ok(Some(pk)) => pk,
        Ok(None) => {
            log("客户端未发送 key_exchange");
            return None;
        }
        Err(_) => {
            log("等待 key_exchange 超时 (10s)");
            return None;
        }
    };

    log(&format!("收到 client_pk，长度: {}", client_pk_b64.len()));

    // 4. 导入客户端公钥 + 计算 shared_secret
    let client_pk = match crypto::import_public_key(&client_pk_b64) {
        Ok(pk) => pk,
        Err(e) => {
            log(&format!("导入客户端公钥失败: {}", e));
            return None;
        }
    };
    let shared_secret = server_sk.diffie_hellman(&client_pk);
    let aes_key = crypto::derive_aes_key(&shared_secret);
    log("AES 密钥派生成功");

    // 5. 发送加密的 key_exchange_ok 确认
    let ok_msg = serde_json::to_string(&WsOutbound::KeyExchangeOk).ok()?;
    let encrypted = crypto::encrypt(&aes_key, ok_msg.as_bytes());
    if ws_tx.send(Message::Binary(encrypted.into())).await.is_err() {
        log("发送 key_exchange_ok 失败");
        return None;
    }
    log("密钥协商完成");

    Some(aes_key)
}

/// 处理 WebSocket 连接（含 ECDH 协商 + AES-256-GCM 加密通信）
async fn handle_websocket(
    ws_stream: tokio_tungstenite::WebSocketStream<TcpStream>,
    inbound_tx: mpsc::Sender<WsInbound>,
    outbound_tx: broadcast::Sender<WsOutbound>,
    client_connected: &Arc<AtomicBool>,
    mut kick_rx: watch::Receiver<u64>,
) {
    let (mut ws_tx, mut ws_rx) = ws_stream.split();
    let mut outbound_rx = outbound_tx.subscribe();

    // ---- ECDH 密钥协商 ----
    let aes_key = match perform_key_exchange(&mut ws_tx, &mut ws_rx).await {
        Some(key) => key,
        None => {
            crate::util::log::write_error_log("[remote::ws]", "ECDH 密钥协商失败或超时，断开连接");
            let _ = ws_tx.send(Message::Close(None)).await;
            client_connected.store(false, Ordering::Relaxed);
            return;
        }
    };

    // ---- 加密通信主循环 ----
    // 服务端主动 ping 定时器
    let mut ping_interval =
        tokio::time::interval(tokio::time::Duration::from_secs(PING_INTERVAL_SECS));
    ping_interval.reset(); // 从现在开始计时

    // pong 超时检测：上次收到客户端任何消息的时间
    let mut last_activity = tokio::time::Instant::now();
    let pong_timeout = tokio::time::Duration::from_secs(PONG_TIMEOUT_SECS);

    // 记录当前 kick 版本，后续检测是否有新连接踢掉自己
    let kick_version = *kick_rx.borrow_and_update();

    loop {
        tokio::select! {
            // 客户端 → 服务端（加密的 Binary 帧）
            msg = ws_rx.next() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        last_activity = tokio::time::Instant::now();
                        // 解密
                        match crypto::decrypt(&aes_key, &data) {
                            Ok(plaintext) => {
                                let text = match String::from_utf8(plaintext) {
                                    Ok(s) => s,
                                    Err(_) => {
                                        crate::util::log::write_error_log(
                                            "[remote::ws]",
                                            "解密后数据非 UTF-8",
                                        );
                                        continue;
                                    }
                                };
                                match serde_json::from_str::<WsInbound>(&text) {
                                    Ok(inbound) => {
                                        if inbound_tx.send(inbound).await.is_err() {
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        // 发送加密的错误消息
                                        let err_msg = WsOutbound::Error {
                                            message: format!("解析消息失败: {}", e),
                                        };
                                        if let Ok(json) = serde_json::to_string(&err_msg) {
                                            let enc = crypto::encrypt(&aes_key, json.as_bytes());
                                            let _ = ws_tx.send(Message::Binary(enc.into())).await;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                crate::util::log::write_error_log(
                                    "[remote::ws]",
                                    &format!("消息解密失败: {}", e),
                                );
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        last_activity = tokio::time::Instant::now();
                        let _ = ws_tx.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Pong(_))) => {
                        last_activity = tokio::time::Instant::now();
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            // 服务端 → 客户端（加密发送）
            msg = outbound_rx.recv() => {
                match msg {
                    Ok(outbound) => {
                        if let Ok(json) = serde_json::to_string(&outbound) {
                            let encrypted = crypto::encrypt(&aes_key, json.as_bytes());
                            if ws_tx.send(Message::Binary(encrypted.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        crate::util::log::write_info_log(
                            "[remote::ws]",
                            &format!("客户端落后 {} 条消息", n),
                        );
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            // 服务端主动发 ping，并检测 pong 超时
            _ = ping_interval.tick() => {
                // 检查是否超时（上次活动距今超过阈值）
                if last_activity.elapsed() > pong_timeout {
                    crate::util::log::write_info_log(
                        "[remote::ws]",
                        "客户端 pong 超时，断开连接",
                    );
                    let _ = ws_tx.send(Message::Close(None)).await;
                    break;
                }
                // 发送 ping
                let _ = ws_tx.send(Message::Ping(vec![].into())).await;
            }
            // 被新连接踢掉
            _ = kick_rx.changed() => {
                if *kick_rx.borrow() != kick_version {
                    crate::util::log::write_info_log(
                        "[remote::ws]",
                        "新客户端连接，踢掉旧连接",
                    );
                    let _ = ws_tx.send(Message::Close(None)).await;
                    break;
                }
            }
        }
    }

    client_connected.store(false, Ordering::Relaxed);
}

/// 从请求字符串中提取查询参数
fn extract_query_param(request: &str, key: &str) -> Option<String> {
    let first_line = request.lines().next()?;
    let path = first_line.split_whitespace().nth(1)?;
    let query = path.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        if let (Some(k), Some(v)) = (kv.next(), kv.next())
            && k == key
        {
            return Some(v.to_string());
        }
    }
    None
}

/// 返回嵌入的 HTML 页面
async fn serve_html(mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::AsyncWriteExt;

    // 先消费掉 peek 过的请求数据
    let mut discard = vec![0u8; 4096];
    loop {
        let n = stream.try_read(&mut discard).unwrap_or(0);
        if n == 0 {
            break;
        }
    }

    let html = Assets::get("remote.html")
        .map(|f| f.data.to_vec())
        .unwrap_or_else(|| b"<h1>remote.html not found</h1>".to_vec());

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        html.len()
    );

    stream.write_all(response.as_bytes()).await?;
    stream.write_all(&html).await?;
    stream.flush().await?;
    Ok(())
}

/// 返回 HTTP 错误响应
async fn serve_error(
    mut stream: TcpStream,
    status: u16,
    body: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::AsyncWriteExt;

    // 先消费掉 peek 过的请求数据
    let mut discard = vec![0u8; 4096];
    loop {
        let n = stream.try_read(&mut discard).unwrap_or(0);
        if n == 0 {
            break;
        }
    }

    let response = format!(
        "HTTP/1.1 {} Error\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        body.len(),
        body
    );

    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}
