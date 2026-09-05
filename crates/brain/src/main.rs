mod io;

use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use protocol::{ClickState, KeyState, Message, MouseButton};
use tokio::time::sleep;
use tracing::{info, warn};

use crate::io::AgentClient;

#[derive(Parser, Debug)]
#[command(name = "brain", about = "Mac-side brain for rust-outpost-bot")]
struct Args {
    /// PC agent address, e.g. 192.168.1.42:7878
    #[arg(long, env = "BRAIN_AGENT")]
    agent: String,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Connect, send heartbeats, do nothing else. Useful to verify the link.
    Ping,
    /// Run the built-in smoke sequence: walk forward, look right, click.
    Demo,
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
    let mut client = AgentClient::connect(&args.agent)
        .await
        .with_context(|| format!("connect {}", args.agent))?;
    info!(agent = %args.agent, "connected to agent");

    match args.cmd {
        Cmd::Ping => run_ping(&mut client).await,
        Cmd::Demo => run_demo(&mut client).await,
    }
}

async fn run_ping(client: &mut AgentClient) -> Result<()> {
    loop {
        client.send(Message::heartbeat()).await?;
        sleep(Duration::from_millis(500)).await;
    }
}

async fn run_demo(client: &mut AgentClient) -> Result<()> {
    let hb = client.spawn_heartbeat(Duration::from_millis(500));

    // 3-second countdown so the operator can focus the game window.
    for i in (1..=3).rev() {
        info!("demo starts in {i}…");
        sleep(Duration::from_secs(1)).await;
    }

    info!("walk forward 2s");
    client.send(Message::key("w", KeyState::Down)).await?;
    sleep(Duration::from_secs(2)).await;
    client.send(Message::key("w", KeyState::Up)).await?;

    info!("look right (500 px over ~50 ticks)");
    for _ in 0..50 {
        client.send(Message::mouse_move(10, 0)).await?;
        sleep(Duration::from_millis(20)).await;
    }

    info!("left click");
    client.send(Message::mouse(MouseButton::Left, ClickState::Click)).await?;

    info!("demo complete");
    hb.abort();
    // Give the agent a beat to flush before we drop the socket.
    sleep(Duration::from_millis(200)).await;
    if let Err(e) = client.send(Message::heartbeat()).await {
        warn!(?e, "final heartbeat failed");
    }
    Ok(())
}
