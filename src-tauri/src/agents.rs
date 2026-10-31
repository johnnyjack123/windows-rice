//! Coding agents ask for permission in the island (docs/BACKEND.md §4
//! "Coding-Agents"). Hook processes (`rice.exe --agent-hook claude`, see
//! `agent_hook.rs`) connect to a named pipe only this user can open. A
//! permission request waits until the user decides, the hook goes away
//! (answered in the terminal, timed out) or [`GIVE_UP`] passes; rice never
//! allows anything on its own.

use crate::state::{AppState, Locked};
use rice_core::agents::{self, Decision, MAX_MESSAGE, PROTOCOL, PipeAnswer, PipeMessage};
use rice_core::contract::{AgentId, AgentIntegration, AgentRequest, AgentStatus, events};
use rice_core::time::now_iso;
use rice_core::{BackendError, BackendResult};
use rice_services::fsutil::atomic_write;
use rice_win::pipes::{UserOnly, agent_pipe_name};
use serde_json::{Value, json};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, ReadHalf};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::oneshot;

/// Longest a request waits; the hook's own timeout (600 s) normally ends it first.
const GIVE_UP: Duration = Duration::from_secs(15 * 60);

/// Requests waiting for the user, oldest first.
#[derive(Default)]
pub struct Agents {
    pending: Mutex<Vec<Pending>>,
    next_id: AtomicU64,
}

struct Pending {
    request: AgentRequest,
    answer: oneshot::Sender<Decision>,
}

impl Agents {
    fn take(&self, id: &str) -> Option<Pending> {
        let mut pending = self.pending.locked();
        let index = pending.iter().position(|p| p.request.id == id)?;
        Some(pending.remove(index))
    }
}

/// Opens the pipe for hook processes.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = serve(&app).await {
            log::warn!("agents: pipe server stopped ({error})");
        }
    });
}

async fn serve(app: &AppHandle) -> io::Result<()> {
    let name = agent_pipe_name();
    let mut security = UserOnly::new().map_err(io::Error::other)?;
    // The first instance fails if another program already holds the name.
    let mut server = create(&name, &mut security, true)?;
    loop {
        if let Err(error) = server.connect().await {
            // E.g. a client that left before we accepted it.
            log::debug!("agents: connect failed ({error})");
            server = create(&name, &mut security, false)?;
            continue;
        }
        let client = std::mem::replace(&mut server, create(&name, &mut security, false)?);
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = converse(&app, client).await {
                log::debug!("agents: {error}");
            }
        });
    }
}

fn create(name: &str, security: &mut UserOnly, first: bool) -> io::Result<NamedPipeServer> {
    let mut options = ServerOptions::new();
    options.first_pipe_instance(first).reject_remote_clients(true);
    // SAFETY: `security` outlives the call; the system copies the descriptor.
    unsafe { options.create_with_security_attributes_raw(name, security.as_mut_ptr()) }
}

/// One connection: one message, and for a permission one answer.
async fn converse(app: &AppHandle, pipe: NamedPipeServer) -> io::Result<()> {
    let (read, mut write) = tokio::io::split(pipe);
    let mut reader = BufReader::new(read);
    let mut line = Vec::new();
    (&mut reader).take(MAX_MESSAGE).read_until(b'\n', &mut line).await?;
    let message: PipeMessage = serde_json::from_slice(&line).map_err(io::Error::other)?;
    match message {
        PipeMessage::Status { v, agent, cwd, state, message, .. } => {
            if let Some(agent) = known(v, &agent) {
                let status = AgentStatus { agent, project: agents::project_name(&cwd), state, message, time: now_iso() };
                let _ = app.emit(events::AGENTS_STATUS, status);
            }
            Ok(())
        }
        PipeMessage::Permission { v, agent, cwd, tool, input, .. } => {
            let decision = match known(v, &agent) {
                Some(agent) => {
                    let (kind, detail) = agents::describe(&tool, &input);
                    let request =
                        AgentRequest { id: String::new(), agent, project: agents::project_name(&cwd), tool, kind, detail, time: now_iso() };
                    ask(app, request, reader).await
                }
                None => Decision::Terminal,
            };
            let mut answer = serde_json::to_vec(&PipeAnswer { decision }).map_err(io::Error::other)?;
            answer.push(b'\n');
            // The hook may be gone already; then nobody needs the answer.
            write.write_all(&answer).await?;
            write.flush().await
        }
    }
}

/// The agent of a message in a protocol version rice speaks.
fn known(version: u32, agent: &str) -> Option<AgentId> {
    (version == PROTOCOL).then(|| serde_json::from_value(json!(agent)).ok()).flatten()
}

/// Shows `request` and waits for the user, the hook leaving, or [`GIVE_UP`].
async fn ask(app: &AppHandle, mut request: AgentRequest, mut reader: BufReader<ReadHalf<NamedPipeServer>>) -> Decision {
    let state = app.state::<AppState>();
    let id = format!("a{}", state.agents.next_id.fetch_add(1, Ordering::Relaxed) + 1);
    request.id.clone_from(&id);
    let (sender, answer) = oneshot::channel();
    state.agents.pending.locked().push(Pending { request: request.clone(), answer: sender });
    let _ = app.emit(events::AGENTS_REQUEST, request);
    let mut rest = [0u8; 64];
    let decision = tokio::select! {
        decision = answer => decision.unwrap_or(Decision::Terminal),
        // End of input: the hook was answered in the terminal, cancelled or killed.
        () = async { while matches!(reader.read(&mut rest).await, Ok(n) if n > 0) {} } => Decision::Terminal,
        () = tokio::time::sleep(GIVE_UP) => Decision::Terminal,
    };
    state.agents.take(&id);
    let _ = app.emit(events::AGENTS_RESOLVED, json!({ "id": id }));
    decision
}

/// `agents_pending`
pub fn pending(app: &AppHandle) -> Vec<AgentRequest> {
    app.state::<AppState>().agents.pending.locked().iter().map(|p| p.request.clone()).collect()
}

/// `agents_respond`: only ever called for a click in the island.
pub fn respond(app: &AppHandle, id: &str, decision: Decision) -> BackendResult<()> {
    let pending = app.state::<AppState>().agents.take(id).ok_or_else(|| BackendError::not_found(format!("no request {id}")))?;
    // The waiting task emits `agents://resolved`.
    let _ = pending.answer.send(decision);
    Ok(())
}

/// `agents_list`. OpenCode and Pi follow in a later step.
pub fn list() -> Vec<AgentIntegration> {
    vec![claude_integration()]
}

/// `agents_connect`
pub fn connect(id: AgentId) -> BackendResult<AgentIntegration> {
    match id {
        AgentId::Claude => {
            let exe = std::env::current_exe().map_err(BackendError::internal)?;
            let exe = exe.to_string_lossy();
            edit_settings(&claude_settings()?, |settings| agents::claude_connect(settings, &exe).map(|()| true))?;
            Ok(claude_integration())
        }
        AgentId::OpenCode | AgentId::Pi => Err(BackendError::not_supported(format!("{id:?} is not supported yet"))),
    }
}

/// `agents_disconnect`
pub fn disconnect(id: AgentId) -> BackendResult<AgentIntegration> {
    match id {
        AgentId::Claude => {
            edit_settings(&claude_settings()?, |settings| Ok(agents::claude_disconnect(settings)))?;
            Ok(claude_integration())
        }
        AgentId::OpenCode | AgentId::Pi => Err(BackendError::not_supported(format!("{id:?} is not supported yet"))),
    }
}

/// For the uninstaller (`rice.exe --disconnect-agents`): removes every hook.
pub fn disconnect_all() {
    if let Err(error) = disconnect(AgentId::Claude) {
        eprintln!("rice: {}", error.message);
    }
}

/// Claude Code's user settings: `%CLAUDE_CONFIG_DIR%` or `%USERPROFILE%\.claude`.
fn claude_settings() -> BackendResult<PathBuf> {
    let dir = match std::env::var_os("CLAUDE_CONFIG_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(std::env::var_os("USERPROFILE").ok_or_else(|| BackendError::not_supported("no user profile"))?).join(".claude"),
    };
    Ok(dir.join("settings.json"))
}

fn claude_integration() -> AgentIntegration {
    let path = claude_settings().ok();
    let available = path.as_deref().and_then(Path::parent).is_some_and(Path::is_dir);
    let connected = path.and_then(|p| read_settings(&p).ok()).is_some_and(|(settings, _)| agents::claude_connected(&settings));
    AgentIntegration { id: AgentId::Claude, available, connected }
}

/// A JSON settings file and its bytes; a missing file is an empty object.
fn read_settings(path: &Path) -> BackendResult<(Value, Option<Vec<u8>>)> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok((json!({}), None)),
        Err(error) => return Err(error.into()),
    };
    let text = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    let settings = serde_json::from_slice(text)
        .map_err(|error| BackendError::invalid_args(format!("{} is not valid JSON ({error})", path.display())))?;
    Ok((settings, Some(bytes)))
}

/// Changes a settings file: a backup of the old file next to it, then an
/// atomic write. An unreadable file is never overwritten.
fn edit_settings(path: &Path, change: impl FnOnce(&mut Value) -> Result<bool, String>) -> BackendResult<()> {
    let (mut settings, original) = read_settings(path)?;
    if !change(&mut settings).map_err(BackendError::invalid_args)? {
        return Ok(());
    }
    // Dotfile setups link settings.json elsewhere; write the real file.
    let target = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
    if let Some(bytes) = original {
        let mut backup = target.clone().into_os_string();
        backup.push(".rice-backup");
        atomic_write(Path::new(&backup), &bytes)?;
    }
    let mut text = serde_json::to_string_pretty(&settings).map_err(BackendError::internal)?;
    text.push('\n');
    atomic_write(&target, text.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_settings_with_a_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let backup = dir.path().join("settings.json.rice-backup");

        // A missing file becomes one with only rice's hooks, without a backup.
        edit_settings(&path, |s| agents::claude_connect(s, "rice.exe").map(|()| true)).unwrap();
        assert!(agents::claude_connected(&read_settings(&path).unwrap().0));
        assert!(!backup.exists());

        // An existing file (with BOM) is backed up byte for byte; order and keys survive.
        let original = "\u{feff}{\n  \"model\": \"opus\",\n  \"env\": { \"A\": \"1\" }\n}";
        std::fs::write(&path, original).unwrap();
        edit_settings(&path, |s| agents::claude_connect(s, "rice.exe").map(|()| true)).unwrap();
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), original);
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.starts_with("{\n  \"model\": \"opus\",\n  \"env\""), "{written}");
        edit_settings(&path, |s| Ok(agents::claude_disconnect(s))).unwrap();
        assert_eq!(read_settings(&path).unwrap().0, json!({ "model": "opus", "env": { "A": "1" } }));

        // Nothing to change: no write. Broken JSON: no write either.
        let before = std::fs::read(&path).unwrap();
        edit_settings(&path, |s| Ok(agents::claude_disconnect(s))).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), before);
        std::fs::write(&path, "{ broken").unwrap();
        assert!(edit_settings(&path, |s| agents::claude_connect(s, "rice.exe").map(|()| true)).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ broken");
    }

    #[test]
    fn hooks_talk_through_a_pipe_only_one_rice_owns() {
        let name = format!(r"\\.\pipe\rice-agents-test-{}", std::process::id());
        let message = PipeMessage::Permission {
            v: PROTOCOL,
            agent: "claude".into(),
            session: "s".into(),
            cwd: r"C:\rice".into(),
            tool: "Bash".into(),
            input: json!({ "command": "ls" }),
        };
        let mut security = UserOnly::new().unwrap();
        tauri::async_runtime::block_on(async {
            let server = create(&name, &mut security, true).unwrap();
            // A second program cannot take the name while rice holds it.
            assert!(create(&name, &mut security, true).is_err());
            // The test binary serves and calls, so the executable check passes.
            let (client_name, sent) = (name.clone(), message.clone());
            let client = tauri::async_runtime::spawn_blocking(move || crate::agent_hook::exchange(&client_name, &sent));
            server.connect().await.unwrap();
            let (read, mut write) = tokio::io::split(server);
            let mut line = Vec::new();
            BufReader::new(read).take(MAX_MESSAGE).read_until(b'\n', &mut line).await.unwrap();
            assert_eq!(serde_json::from_slice::<PipeMessage>(&line).unwrap(), message);
            write.write_all(b"{\"decision\":\"allow\"}\n").await.unwrap();
            // Like `converse`: the server end closes right after answering.
            drop(write);
            assert_eq!(client.await.unwrap(), Some(PipeAnswer { decision: Decision::Allow }));
        });
        // Nobody serves the name any more: the hook gives up at once.
        assert_eq!(crate::agent_hook::exchange(&name, &message), None);
    }

    #[test]
    fn knows_agents_of_the_current_protocol() {
        assert_eq!(known(PROTOCOL, "claude"), Some(AgentId::Claude));
        assert_eq!(known(PROTOCOL, "opencode"), Some(AgentId::OpenCode));
        assert_eq!(known(PROTOCOL, "codex"), None);
        assert_eq!(known(PROTOCOL + 1, "claude"), None);
    }
}
