use crossterm::event::{self, Event};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;

/// 独立输入线程：持续从 crossterm 读取键盘/鼠标/Resize 事件，放入 channel。
///
/// 主循环只从 `rx` 取事件，无论渲染多慢，输入永远不丢。
/// 编辑器操作时通过 `pause()` / `resume()` 暂停/恢复读取。
pub struct InputThread {
    /// 事件接收端（主循环消费）
    pub rx: mpsc::Receiver<Event>,
    /// 退出标志
    quit_flag: Arc<AtomicBool>,
    /// 暂停标志（编辑器独占 stdin 时设置）
    pause_flag: Arc<AtomicBool>,
    /// 后台线程句柄
    handle: Option<JoinHandle<()>>,
}

impl InputThread {
    /// 启动输入线程，返回 InputThread 实例
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<Event>();
        let quit_flag = Arc::new(AtomicBool::new(false));
        let pause_flag = Arc::new(AtomicBool::new(false));

        let quit = Arc::clone(&quit_flag);
        let pause = Arc::clone(&pause_flag);

        let handle = std::thread::Builder::new()
            .name("input-thread".into())
            .spawn(move || {
                Self::run_loop(tx, quit, pause);
            })
            .expect("failed to spawn input thread");

        Self {
            rx,
            quit_flag,
            pause_flag,
            handle: Some(handle),
        }
    }

    /// 线程内部循环
    fn run_loop(tx: mpsc::Sender<Event>, quit: Arc<AtomicBool>, pause: Arc<AtomicBool>) {
        while !quit.load(Ordering::Relaxed) {
            if pause.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(50));
                continue;
            }
            // poll 50ms：既保证响应退出/暂停信号，又不会忙等
            match event::poll(std::time::Duration::from_millis(50)) {
                Ok(true) => {
                    // 二次检查 pause，避免 poll 返回 true 后 pause 已被设置
                    if pause.load(Ordering::Relaxed) {
                        continue;
                    }
                    match event::read() {
                        Ok(evt) => {
                            if tx.send(evt).is_err() {
                                // 主线程已 drop rx，退出
                                break;
                            }
                        }
                        Err(_) => {
                            // read 出错（极罕见），短暂休眠后重试
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                    }
                }
                Ok(false) => {
                    // poll 超时，无事件，继续循环
                }
                Err(_) => {
                    // poll 出错，短暂休眠后重试
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }
    }

    /// 暂停输入线程（编辑器需要独占 stdin 时调用）
    ///
    /// 设置 pause_flag 后 sleep 60ms，确保线程退出当前 poll 周期
    pub fn pause(&self) {
        self.pause_flag.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(60));
    }

    /// 恢复输入线程
    pub fn resume(&self) {
        self.pause_flag.store(false, Ordering::Relaxed);
    }

    /// 清空 channel 中的残留事件
    pub fn drain(&self) {
        while self.rx.try_recv().is_ok() {}
    }

    /// 停止输入线程并等待其结束
    pub fn shutdown(mut self) {
        self.stop_inner();
    }

    /// 内部停止逻辑（shutdown 和 Drop 共用）
    fn stop_inner(&mut self) {
        self.quit_flag.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for InputThread {
    fn drop(&mut self) {
        self.stop_inner();
    }
}
