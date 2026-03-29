//! HTTP + WebSocket 混合服务器

use super::protocol::{WsInbound, WsOutbound};
use crate::assets::Assets;
use futures::SinkExt;
use futures::stream::StreamExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, broadcast, mpsc};
use tokio_tungstenite::tungstenite::protocol::Message;

/// 启动 HTTP + WS 服务器
///
/// - `GET /` → 返回嵌入的 remote.html
/// - `GET /ws?token=xxx` → WebSocket 升级
pub async fn run_server(
    listener: TcpListener,
    token: String,
    inbound_tx: mpsc::Sender<WsInbound>,
    outbound_tx: broadcast::Sender<WsOutbound>,
    client_connected: Arc<AtomicBool>,
    client_notify: Arc<Notify>,
) {
    loop {
        let Ok((stream, _addr)) = listener.accept().await else {
            continue;
        };

        let token = token.clone();
        let inbound_tx = inbound_tx.clone();
        let outbound_tx = outbound_tx.clone();
        let client_connected = Arc::clone(&client_connected);
        let client_notify = Arc::clone(&client_notify);

        tokio::spawn(async move {
            if let Err(e) = handle_connection(
                stream,
                &token,
                inbound_tx,
                outbound_tx,
                client_connected,
                client_notify,
            )
            .await
            {
                crate::util::log::write_error_log(
                    "[remote::server]",
                    &format!("连接处理错误: {}", e),
                );
            }
        });
    }
}

async fn handle_connection(
    stream: TcpStream,
    token: &str,
    inbound_tx: mpsc::Sender<WsInbound>,
    outbound_tx: broadcast::Sender<WsOutbound>,
    client_connected: Arc<AtomicBool>,
    client_notify: Arc<Notify>,
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

        // 单客户端限制
        if client_connected.load(Ordering::Relaxed) {
            serve_error(stream, 409, "Conflict: another client already connected").await?;
            return Ok(());
        }

        // WebSocket 升级
        let ws_stream = tokio_tungstenite::accept_async(stream).await?;
        client_connected.store(true, Ordering::Relaxed);
        client_notify.notify_one();

        handle_websocket(ws_stream, inbound_tx, outbound_tx, &client_connected).await;

        client_connected.store(false, Ordering::Relaxed);
        return Ok(());
    }

    // 未知路径
    serve_error(stream, 404, "Not Found").await?;
    Ok(())
}

/// 处理 WebSocket 连接
async fn handle_websocket(
    ws_stream: tokio_tungstenite::WebSocketStream<TcpStream>,
    inbound_tx: mpsc::Sender<WsInbound>,
    outbound_tx: broadcast::Sender<WsOutbound>,
    client_connected: &Arc<AtomicBool>,
) {
    let (mut ws_tx, mut ws_rx) = ws_stream.split();
    let mut outbound_rx = outbound_tx.subscribe();

    loop {
        tokio::select! {
            // 客户端 → 服务端
            msg = ws_rx.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<WsInbound>(&text) {
                            Ok(inbound) => {
                                if inbound_tx.send(inbound).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                let err_msg = WsOutbound::Error {
                                    message: format!("解析消息失败: {}", e),
                                };
                                let _ = ws_tx.send(Message::Text(
                                    serde_json::to_string(&err_msg).unwrap_or_default().into()
                                )).await;
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = ws_tx.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            // 服务端 → 客户端
            msg = outbound_rx.recv() => {
                match msg {
                    Ok(outbound) => {
                        if let Ok(json) = serde_json::to_string(&outbound)
                            && ws_tx.send(Message::Text(json.into())).await.is_err()
                        {
                            break;
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
