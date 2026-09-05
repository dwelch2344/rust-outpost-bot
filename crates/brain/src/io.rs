use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use protocol::Message;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::warn;

#[derive(Clone)]
pub struct AgentClient {
    inner: Arc<Mutex<TcpStream>>,
}

impl AgentClient {
    pub async fn connect(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        stream.set_nodelay(true).ok();
        Ok(Self { inner: Arc::new(Mutex::new(stream)) })
    }

    pub async fn send(&self, msg: Message) -> Result<()> {
        let mut buf = serde_json::to_vec(&msg)?;
        buf.push(b'\n');
        let mut guard = self.inner.lock().await;
        guard.write_all(&buf).await?;
        Ok(())
    }

    /// Fire a heartbeat every `period`; caller aborts the handle to stop.
    pub fn spawn_heartbeat(&self, period: Duration) -> JoinHandle<()> {
        let client = self.clone();
        tokio::spawn(async move {
            let mut ticker = interval(period);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                if let Err(e) = client.send(Message::heartbeat()).await {
                    warn!(?e, "heartbeat send failed");
                    return;
                }
            }
        })
    }
}
