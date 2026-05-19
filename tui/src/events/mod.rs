//! 事件模块：把键盘、鼠标、后台 API 结果统一转换为 AppEvent。

use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Tick,
    Log(String),
    DelayResult {
        provider: String,
        node: String,
        delay: Option<u64>,
    },
    ProviderPingDone(String),
}

// #----输入事件----
pub fn spawn_input(tx: mpsc::UnboundedSender<AppEvent>) {
    std::thread::spawn(move || loop {
        if event::poll(Duration::from_millis(150)).unwrap_or(false) {
            match event::read() {
                Ok(CrosstermEvent::Key(key)) => {
                    let _ = tx.send(AppEvent::Key(key));
                }
                Ok(CrosstermEvent::Mouse(mouse)) => {
                    let _ = tx.send(AppEvent::Mouse(mouse));
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        let _ = tx.send(AppEvent::Tick);
    });
}

pub fn spawn_log_bridge(
    mut log_rx: mpsc::UnboundedReceiver<String>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
) {
    tokio::spawn(async move {
        while let Some(line) = log_rx.recv().await {
            let _ = event_tx.send(AppEvent::Log(line));
        }
    });
}
