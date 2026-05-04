use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;

struct Blackbox {
    session_id: String,
    session_dir: PathBuf,
    machine_log: File,
    recent_bridge_messages: VecDeque<String>,
}

impl Blackbox {
    fn new() -> Result<Self> {
        let session_id = Uuid::new_v4().to_string();
        let session_dir = PathBuf::from("../logs").join(&session_id);
        fs::create_dir_all(&session_dir)?;

        let machine_log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(session_dir.join("machine.jsonl"))?;

        Ok(Self {
            session_id,
            session_dir,
            machine_log,
            recent_bridge_messages: VecDeque::with_capacity(10),
        })
    }

    fn log(&mut self, component: &str, event_type: &str, payload: Value) -> Result<()> {
        let event = json!({
            "timestamp": Utc::now().to_rfc3339(),
                          "session_id": self.session_id,
                          "component": component,
                          "event_type": event_type,
                          "payload": payload
        });

        writeln!(self.machine_log, "{event}")?;
        self.machine_log.flush()?;
        Ok(())
    }

    fn note_bridge_message(&mut self, line: &str) {
        if self.recent_bridge_messages.len() == 10 {
            self.recent_bridge_messages.pop_front();
        }
        self.recent_bridge_messages.push_back(line.to_string());
    }

    fn write_summary(&self, status: &str, notes: &[String]) -> Result<()> {
        let mut f = File::create(self.session_dir.join("summary.md"))?;
        writeln!(f, "# NickVM Session Summary")?;
        writeln!(f, "- Session ID: {}", self.session_id)?;
        writeln!(f, "- Status: {}", status)?;
        writeln!(f)?;
        writeln!(f, "## Notes")?;
        for n in notes {
            writeln!(f, "- {}", n)?;
        }
        writeln!(f)?;
        writeln!(f, "## Last Bridge Messages")?;
        for msg in &self.recent_bridge_messages {
            writeln!(f, "- {}", msg)?;
        }
        Ok(())
    }
}

struct Bridge {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Bridge {
    fn init() -> Result<Self> {
        let mut child = Command::new("python3")
        .arg("../runtime/current_runtime.py")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to spawn python runtime")?;

        let stdin = child.stdin.take().context("missing child stdin")?;
        let stdout = child.stdout.take().context("missing child stdout")?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    fn send(&mut self, msg: &Value) -> Result<()> {
        writeln!(self.stdin, "{}", serde_json::to_string(msg)?)?;
        self.stdin.flush()?;
        Ok(())
    }

    fn recv_line(&mut self) -> Result<String> {
        let mut line = String::new();
        self.stdout.read_line(&mut line)?;
        if line.trim().is_empty() {
            anyhow::bail!("bridge received empty line or EOF");
        }
        Ok(line.trim().to_string())
    }

    fn recv_json(&mut self) -> Result<Value> {
        let line = self.recv_line()?;
        let value: Value = serde_json::from_str(&line)
        .with_context(|| format!("invalid json from runtime: {line}"))?;
        Ok(value)
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }

    fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>> {
        Ok(self.child.try_wait()?)
    }
}

fn main() -> Result<()> {
    let mut notes = Vec::<String>::new();
    let mut blackbox = Blackbox::new()?;

    blackbox.log("core", "system_start", json!({}))?;
    notes.push("Rust core started".into());

    let mut bridge = Bridge::init()?;
    blackbox.log("bridge", "python_spawned", json!({ "pid": bridge.pid() }))?;
    notes.push(format!("Python runtime spawned with pid {}", bridge.pid()));

    // 1) mandatory startup hello
    let hello_line = bridge.recv_line()?;
    blackbox.note_bridge_message(&hello_line);
    let hello_json: Value = serde_json::from_str(&hello_line)?;
    blackbox.log("bridge", "hello_received", hello_json.clone())?;
    notes.push("Startup hello handshake received".into());

    // 2) ping/pong validation
    bridge.send(&json!({"type":"ping"}))?;
    blackbox.log("bridge", "message_sent", json!({"type":"ping"}))?;
    let pong_line = bridge.recv_line()?;
    blackbox.note_bridge_message(&pong_line);
    let pong_json: Value = serde_json::from_str(&pong_line)?;
    blackbox.log("bridge", "message_received", pong_json.clone())?;
    notes.push("Ping/pong succeeded".into());

    // 3) registry lookup
    bridge.send(&json!({"type":"whois","name":"theme"}))?;
    blackbox.log("registry", "whois_sent", json!({"name":"theme"}))?;
    let whois_line = bridge.recv_line()?;
    blackbox.note_bridge_message(&whois_line);
    let whois_json: Value = serde_json::from_str(&whois_line)?;
    blackbox.log("registry", "whois_received", whois_json.clone())?;
    notes.push("Registry lookup for theme succeeded".into());

    // 4) theme get
    bridge.send(&json!({"type":"theme_get"}))?;
    let theme_before_line = bridge.recv_line()?;
    blackbox.note_bridge_message(&theme_before_line);
    let theme_before_json: Value = serde_json::from_str(&theme_before_line)?;
    blackbox.log("dripd", "theme_get", theme_before_json.clone())?;
    notes.push("Initial theme fetched".into());

    // 5) theme set
    bridge.send(&json!({"type":"theme_set","accent":"#FF00AA"}))?;
    let theme_after_line = bridge.recv_line()?;
    blackbox.note_bridge_message(&theme_after_line);
    let theme_after_json: Value = serde_json::from_str(&theme_after_line)?;
    blackbox.log("dripd", "theme_set", theme_after_json.clone())?;
    notes.push("Accent color changed to #FF00AA".into());

    // 6) updater scaffold
    bridge.send(&json!({"type":"update_check"}))?;
    let update_line = bridge.recv_line()?;
    blackbox.note_bridge_message(&update_line);
    let update_json: Value = serde_json::from_str(&update_line)?;
    blackbox.log("auntie_up", "update_check", update_json.clone())?;

    notes.push("Updater scaffold check completed".into());

    // 7) tiny heartbeat
    bridge.send(&json!({"type":"ping"}))?;
    blackbox.log("bridge", "heartbeat_ping_sent", json!({}))?;

    let start = Instant::now();
    let hb_line = bridge.recv_line()?;
    let elapsed = start.elapsed();

    blackbox.note_bridge_message(&hb_line);
    let hb_json: Value = serde_json::from_str(&hb_line)?;
    blackbox.log("bridge", "heartbeat_pong_received", json!({
        "elapsed_ms": elapsed.as_millis(),
                                                            "message": hb_json
    }))?;

    if elapsed > Duration::from_secs(1) {
        blackbox.log("bridge", "runtime_hung", json!({
            "reason": "heartbeat timeout",
            "elapsed_ms": elapsed.as_millis()
        }))?;
        notes.push("Heartbeat exceeded timeout".into());
        blackbox.write_summary("hung", &notes)?;
        anyhow::bail!("heartbeat timeout");
    }

    if let Some(status) = bridge.try_wait()? {
        blackbox.log("bridge", "runtime_exited_early", json!({
            "status": status.code()
        }))?;
        notes.push("Python runtime exited early".into());
        blackbox.write_summary("crash", &notes)?;
        anyhow::bail!("runtime exited early");
    }

    notes.push("System stable".into());
    blackbox.log("core", "system_stable", json!({}))?;
    blackbox.write_summary("ok", &notes)?;

    println!("NickOS v0.1 | Session: {}", blackbox.session_id);
    println!("System stable");
    println!("Logs written to ../logs/{}/", blackbox.session_id);

    // tiny pause so output order is easy to read if needed
    thread::sleep(Duration::from_millis(100));

    use std::io::{stdin, stdout};

    println!("\nNickShell v0.1");
    println!("Type 'ping' or 'exit'\n");

    loop {
        let mut input = String::new();

        print!("NickShell> ");
        stdout().flush()?;

        stdin().read_line(&mut input)?;

        let cmd = input.trim();

        if cmd == "exit" {
            println!("Exiting NickShell...");
            break;
        }

        if cmd == "ping" {
            bridge.send(&json!({"type":"ping"}))?;

            let reply = bridge.recv_json()?;

            blackbox.note_bridge_message(&reply.to_string());

            println!("Runtime replied:");
            println!("{}", reply);

            continue;
        }
        if cmd.starts_with("whois ") {
            let name = cmd.trim_start_matches("whois ").trim();

            bridge.send(&json!({"type":"whois","name":name}))?;

            let reply = bridge.recv_json()?;

            blackbox.note_bridge_message(&reply.to_string());

            println!("Registry replied:");
            println!("{}", reply);

            continue;
        }
        println!("Unknown command");
    }

    Ok(())
}
