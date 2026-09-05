# rust-outpost-bot

Vision-driven Rust (Facepunch) Outpost bot, split across two machines on a LAN.
V0 scope and rules: see [issue #1](../../issues/1).

- **PC (game host, Windows)** runs `agent.exe` — a dumb input driver. Listens on
  a TCP port, executes keyboard/mouse commands, releases everything on
  disconnect or heartbeat timeout.
- **Mac (brain)** runs `brain` — connects to the agent, sends commands. V0 ships
  a `demo` subcommand that proves the loop closes; capture/perception/policy
  land in later PRs.

Wire protocol is line-delimited JSON, defined by the `protocol` crate.

## Downloads

Each push to `main` produces artifacts under the **Actions → build** run.
Tagged `v*` pushes cut a **GitHub Release** with:

- `rust-outpost-agent-windows-x64.exe`
- `rust-outpost-brain-macos-arm64`
- `rust-outpost-brain-macos-x86_64`

## Quickstart

### 1. PC (Windows)

1. Download `rust-outpost-agent-windows-x64.exe` from the latest release.
2. Open a terminal (PowerShell or cmd) and run:

   ```powershell
   .\rust-outpost-agent-windows-x64.exe
   ```

   Optional flags:
   ```
   --bind 0.0.0.0:7878              # where to listen (default)
   --heartbeat-timeout-ms 2000      # release all inputs if brain goes silent
   --dry-run                        # log commands, don't drive input
   ```

3. Note the PC's LAN IP (`ipconfig`). Make sure port `7878` is reachable — allow
   it in Windows Firewall on first run.

4. **Safety:** stopping the agent (Ctrl+C, closing the terminal, or unplugging
   the LAN) releases every held key and mouse button. If a heartbeat is not
   received within `--heartbeat-timeout-ms`, the brain is disconnected and
   inputs are released.

### 2. Mac (brain)

1. Download the matching `rust-outpost-brain-macos-*` binary from the release.
2. Make it executable and clear the quarantine bit (unsigned binary):

   ```bash
   chmod +x rust-outpost-brain-macos-arm64
   xattr -d com.apple.quarantine rust-outpost-brain-macos-arm64 2>/dev/null || true
   ```

3. Verify the link:

   ```bash
   ./rust-outpost-brain-macos-arm64 --agent 192.168.1.42:7878 ping
   ```

   You should see `connected to agent` on the Mac and a `brain connected` line
   on the PC. Heartbeats fly every 500 ms.

4. Run the demo (walk forward 2 s → look right → left click). **Focus the Rust
   game window** during the 3-second countdown:

   ```bash
   ./rust-outpost-brain-macos-arm64 --agent 192.168.1.42:7878 demo
   ```

## Layout

```
crates/
  protocol/   # shared wire types (Message, Key/MouseMove/Mouse/Heartbeat)
  agent/      # PC-side input driver (enigo)
  brain/      # Mac-side client + demo; capture/perception/policy land here
```

## Local development

Requires a Rust toolchain (`rustup` recommended). Neither side has been built
locally in the repo yet — CI is the source of truth.

```bash
cargo test -p protocol
cargo run -p brain -- --agent 127.0.0.1:7878 ping
cargo run -p agent -- --dry-run     # safe on any OS
```
