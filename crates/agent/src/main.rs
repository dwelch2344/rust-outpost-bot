mod input;

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use protocol::{Message, DEFAULT_PORT};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::{timeout, Instant};
use tracing::{error, info, warn};

use crate::input::{spawn_input_worker, InputCmd};

#[derive(Parser, Debug)]
#[command(name = "agent", about = "PC-side input agent for rust-outpost-bot")]
struct Args {
    /// Address to bind the command socket on.
    #[arg(long, env = "AGENT_BIND", default_value_t = default_bind())]
    bind: SocketAddr,

    /// Release all inputs if no message is received within this many milliseconds.
    #[arg(long, env = "AGENT_HEARTBEAT_TIMEOUT_MS", default_value_t = 2000)]
    heartbeat_timeout_ms: u64,

    /// Refuse to actually drive input; log commands only. Useful for dry-runs.
    #[arg(long, env = "AGENT_DRY_RUN", default_value_t = false)]
    dry_run: bool,
}

fn default_bind() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    let listener = TcpListener::bind(args.bind)
        .await
        .with_context(|| format!("bind {}", args.bind))?;
    info!(addr = %args.bind, dry_run = args.dry_run, "agent listening");

    let (input_tx, input_rx) = mpsc::channel::<InputCmd>(256);
    spawn_input_worker(input_rx, args.dry_run)?;

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(v) => v,
            Err(e) => {
                error!(?e, "accept failed");
                continue;
            }
        };
        info!(%peer, "brain connected");
        if let Err(e) = handle_conn(stream, &input_tx, Duration::from_millis(args.heartbeat_timeout_ms)).await {
            warn!(%peer, ?e, "connection ended");
        } else {
            info!(%peer, "connection closed");
        }
        // Safety: whenever a brain disconnects, drop all held inputs.
        let _ = input_tx.send(InputCmd::ReleaseAll).await;
    }
}

async fn handle_conn(
    stream: TcpStream,
    input_tx: &mpsc::Sender<InputCmd>,
    heartbeat_timeout: Duration,
) -> Result<()> {
    stream.set_nodelay(true).ok();
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    let mut last_msg = Instant::now();

    loop {
        line.clear();
        let read = timeout(heartbeat_timeout, reader.read_line(&mut line)).await;
        match read {
            Err(_) => {
                warn!(
                    since_last_ms = last_msg.elapsed().as_millis() as u64,
                    "heartbeat timeout — dropping brain"
                );
                return Ok(());
            }
            Ok(Ok(0)) => return Ok(()), // EOF
            Ok(Err(e)) => return Err(e.into()),
            Ok(Ok(_)) => {}
        }

        last_msg = Instant::now();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let msg: Message = match serde_json::from_str(trimmed) {
            Ok(m) => m,
            Err(e) => {
                warn!(?e, raw = %trimmed, "bad message");
                continue;
            }
        };

        match msg {
            Message::Heartbeat { .. } => { /* liveness only */ }
            other => {
                if input_tx.send(InputCmd::Message(other)).await.is_err() {
                    return Err(anyhow::anyhow!("input worker died"));
                }
            }
        }
    }
}
