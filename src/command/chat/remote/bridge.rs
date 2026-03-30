//! WsBridge: 主循环与 WebSocket 服务器之间的通道封装

use super::protocol::{WsInbound, WsOutbound};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{broadcast, mpsc};

/// WebSocket 桥接器：连接 TUI 主循环和 WebSocket 服务器
///
/// 持有 tokio Runtime，bridge 被 drop 时 runtime 随之关闭，
/// 保证 WS 服务器在 TUI 退出后不会残留。
pub struct WsBridge {
    /// 接收来自客户端的消息（server → main loop）
    inbound_rx: mpsc::Receiver<WsInbound>,
    /// 广播给所有客户端的消息（main loop → clients）
    outbound_tx: broadcast::Sender<WsOutbound>,
    /// 是否有客户端连接
    pub client_connected: Arc<AtomicBool>,
    /// 持有 tokio runtime，drop 时自动关闭服务器
    _runtime: tokio::runtime::Runtime,
}

impl WsBridge {
    /// 创建新的 WsBridge，返回 (bridge, inbound_tx, outbound_tx)
    pub fn new(
        runtime: tokio::runtime::Runtime,
    ) -> (Self, mpsc::Sender<WsInbound>, broadcast::Sender<WsOutbound>) {
        let (inbound_tx, inbound_rx) = mpsc::channel::<WsInbound>(256);
        let (outbound_tx, _) = broadcast::channel::<WsOutbound>(256);
        let client_connected = Arc::new(AtomicBool::new(false));

        let bridge = Self {
            inbound_rx,
            outbound_tx: outbound_tx.clone(),
            client_connected,
            _runtime: runtime,
        };

        (bridge, inbound_tx, outbound_tx)
    }

    /// 非阻塞尝试接收一条来自客户端的消息
    pub fn try_recv(&mut self) -> Option<WsInbound> {
        self.inbound_rx.try_recv().ok()
    }

    /// 广播消息给所有已连接的客户端
    pub fn broadcast(&self, msg: WsOutbound) {
        let _ = self.outbound_tx.send(msg);
    }

    /// 是否有客户端连接
    pub fn has_client(&self) -> bool {
        self.client_connected.load(Ordering::Relaxed)
    }

    /// 获取 outbound_tx 的克隆（用于订阅）
    #[allow(dead_code)]
    pub fn subscribe_outbound(&self) -> broadcast::Receiver<WsOutbound> {
        self.outbound_tx.subscribe()
    }

    /// 在 runtime 上阻塞执行 future
    pub fn block_on<F: std::future::Future>(&self, future: F) -> F::Output {
        self._runtime.block_on(future)
    }

    /// 在 runtime 上 spawn 一个后台 task
    pub fn spawn<F>(&self, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        self._runtime.spawn(future);
    }
}
