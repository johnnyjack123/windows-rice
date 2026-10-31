//! Coding agents asking for permission through the island
//! (docs/BACKEND.md §4 "Coding-Agents"): the pipe protocol between a hook
//! process and rice, readable descriptions of tool calls, Claude Code's hook
//! payloads and answers, and rice's entries in Claude Code's settings.

use crate::contract::{AgentStatusState, ToolKind};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Version of the pipe protocol (first field of every message).
pub const PROTOCOL: u32 = 1;
/// Longest message a hook may send (tool inputs can carry whole files).
pub const MAX_MESSAGE: u64 = 1024 * 1024;
/// Command-line flag that turns rice.exe into a hook process.
pub const HOOK_FLAG: &str = "--agent-hook";

const DETAIL_MAX: usize = 600;
const SUMMARY_MAX: usize = 160;
const MESSAGE_MAX: usize = 300;

/// One message from a hook process to rice, one JSON line per connection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PipeMessage {
    /// The agent waits for the user's decision; rice answers with a [`PipeAnswer`].
    Permission { v: u32, agent: String, session: String, cwd: String, tool: String, input: Value },
    /// Fire and forget: the agent finished or waits for input.
    Status { v: u32, agent: String, session: String, cwd: String, state: AgentStatusState, message: Option<String> },
}

/// How the user decided in the island.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Allow,
    Deny,
    /// Leave it to the agent's own prompt in the terminal.
    Terminal,
}

/// rice's answer to [`PipeMessage::Permission`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipeAnswer {
    pub decision: Decision,
}

/// What a tool call does, and its most telling argument (command, path, URL).
pub fn describe(tool: &str, input: &Value) -> (ToolKind, String) {
    let field = |names: &[&str]| names.iter().find_map(|n| input[*n].as_str()).unwrap_or_default().to_owned();
    let (kind, detail) = match tool {
        "Bash" | "PowerShell" => (ToolKind::Command, field(&["command"])),
        "Edit" | "MultiEdit" | "NotebookEdit" => (ToolKind::Edit, field(&["file_path", "notebook_path"])),
        "Write" => (ToolKind::Write, field(&["file_path"])),
        "Read" | "Glob" | "Grep" | "LS" => (ToolKind::Read, field(&["file_path", "pattern", "path"])),
        "WebFetch" => (ToolKind::Fetch, field(&["url"])),
        "WebSearch" => (ToolKind::Fetch, field(&["query"])),
        _ => match tool.strip_prefix("mcp__").and_then(|rest| rest.split_once("__")) {
            Some((server, name)) => (ToolKind::Mcp, format!("{server} · {name}")),
            None => (ToolKind::Other, format!("{tool} {}", compact(input))),
        },
    };
    (kind, clip(detail.trim(), DETAIL_MAX))
}

/// The agent's own words for what a tool call does (Claude Code's
/// `description` of a command or subagent).
pub fn summary(input: &Value) -> Option<String> {
    input["description"].as_str().map(str::trim).filter(|s| !s.is_empty()).map(|s| clip(s, SUMMARY_MAX))
}

/// Last folder of a working directory, shown as the project.
pub fn project_name(cwd: &str) -> String {
    cwd.trim_end_matches(['\\', '/']).rsplit(['\\', '/']).next().unwrap_or(cwd).to_owned()
}

fn compact(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Object(map) if map.is_empty() => String::new(),
        other => other.to_string(),
    }
}

/// Cuts `text` after `max` characters (never inside a character).
pub fn clip(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((at, _)) => format!("{}…", &text[..at]),
        None => text.to_owned(),
    }
}

/// The pipe message for a Claude Code hook payload; `None` for events rice
/// does not show.
pub fn claude_message(payload: &Value) -> Option<PipeMessage> {
    let text = |name: &str| payload[name].as_str().unwrap_or_default().to_owned();
    let (agent, session, cwd) = ("claude".to_owned(), text("session_id"), text("cwd"));
    match payload["hook_event_name"].as_str()? {
        "PermissionRequest" => Some(PipeMessage::Permission {
            v: PROTOCOL,
            agent,
            session,
            cwd,
            tool: payload["tool_name"].as_str()?.to_owned(),
            input: payload["tool_input"].clone(),
        }),
        "Stop" => Some(PipeMessage::Status {
            v: PROTOCOL,
            agent,
            session,
            cwd,
            state: AgentStatusState::Done,
            message: payload["last_assistant_message"].as_str().map(|m| clip(m.trim(), MESSAGE_MAX)).filter(|m| !m.is_empty()),
        }),
        "Notification" if payload["notification_type"].as_str() == Some("idle_prompt") => Some(PipeMessage::Status {
            v: PROTOCOL,
            agent,
            session,
            cwd,
            state: AgentStatusState::Waiting,
            message: None,
        }),
        _ => None,
    }
}

/// What the hook prints for Claude Code; `None` leaves the decision to
/// Claude Code's own prompt.
pub fn claude_answer(decision: Decision) -> Option<Value> {
    let decision = match decision {
        Decision::Allow => json!({ "behavior": "allow" }),
        Decision::Deny => json!({ "behavior": "deny", "message": "The user denied this in rice." }),
        Decision::Terminal => return None,
    };
    Some(json!({ "hookSpecificOutput": { "hookEventName": "PermissionRequest", "decision": decision } }))
}

/// Hook events rice registers in Claude Code: (event, matcher, timeout in s).
/// A permission may wait for the user; the others only report.
const CLAUDE_HOOKS: [(&str, Option<&str>, u64); 3] =
    [("PermissionRequest", Some("*"), 600), ("Stop", None, 10), ("Notification", Some("idle_prompt"), 10)];

/// Whether a hook handler is one of rice's.
fn is_ours(handler: &Value) -> bool {
    handler["args"].as_array().is_some_and(|args| args.first().and_then(Value::as_str) == Some(HOOK_FLAG))
}

/// Whether Claude Code's settings contain rice's hooks.
pub fn claude_connected(settings: &Value) -> bool {
    settings["hooks"].as_object().is_some_and(|hooks| {
        hooks.values().filter_map(Value::as_array).flatten().filter_map(|group| group["hooks"].as_array()).flatten().any(is_ours)
    })
}

/// Adds rice's hooks (exec form: `exe --agent-hook claude`, no shell), after
/// removing older ones. Everything else stays as it is.
pub fn claude_connect(settings: &mut Value, exe: &str) -> Result<(), String> {
    claude_disconnect(settings);
    let root = settings.as_object_mut().ok_or("settings.json is not a JSON object")?;
    let hooks = root.entry("hooks").or_insert_with(|| json!({})).as_object_mut().ok_or("\"hooks\" is not an object")?;
    for (event, matcher, timeout) in CLAUDE_HOOKS {
        let groups = hooks.entry(event).or_insert_with(|| json!([])).as_array_mut().ok_or(format!("\"hooks.{event}\" is not a list"))?;
        let handler = json!({ "type": "command", "command": exe, "args": [HOOK_FLAG, "claude"], "timeout": timeout });
        let mut group = json!({ "hooks": [handler] });
        if let Some(matcher) = matcher {
            group["matcher"] = json!(matcher);
        }
        groups.push(group);
    }
    Ok(())
}

/// Removes rice's hooks, and the groups and lists that only held them.
pub fn claude_disconnect(settings: &mut Value) -> bool {
    let Some(hooks) = settings.get_mut("hooks").and_then(Value::as_object_mut) else {
        return false;
    };
    let mut changed = false;
    let mut emptied = Vec::new();
    for (event, groups) in hooks.iter_mut() {
        let Some(groups) = groups.as_array_mut() else {
            continue;
        };
        let mut removed = false;
        groups.retain_mut(|group| {
            let Some(handlers) = group.get_mut("hooks").and_then(Value::as_array_mut) else {
                return true;
            };
            let before = handlers.len();
            handlers.retain(|handler| !is_ours(handler));
            removed |= handlers.len() != before;
            // Groups that only held rice's hooks go; others keep their remaining hooks.
            !(handlers.is_empty() && before > 0)
        });
        changed |= removed;
        // Only lists that rice emptied go; a list the user left empty stays.
        if removed && groups.is_empty() {
            emptied.push(event.clone());
        }
    }
    for event in &emptied {
        hooks.remove(event);
    }
    if !emptied.is_empty()
        && hooks.is_empty()
        && let Some(root) = settings.as_object_mut()
    {
        root.remove("hooks");
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_tool_calls() {
        assert_eq!(describe("Bash", &json!({ "command": " npm test " })), (ToolKind::Command, "npm test".into()));
        assert_eq!(describe("Edit", &json!({ "file_path": r"C:\x\a.rs" })), (ToolKind::Edit, r"C:\x\a.rs".into()));
        assert_eq!(describe("WebFetch", &json!({ "url": "https://x.dev" })), (ToolKind::Fetch, "https://x.dev".into()));
        assert_eq!(describe("mcp__github__create_issue", &json!({})), (ToolKind::Mcp, "github · create_issue".into()));
        assert_eq!(describe("Task", &json!({ "a": 1 })), (ToolKind::Other, r#"Task {"a":1}"#.into()));
        let (_, long) = describe("Bash", &json!({ "command": "x".repeat(700) }));
        assert_eq!(long.chars().count(), DETAIL_MAX + 1);
        assert_eq!(summary(&json!({ "command": "ls", "description": " List files " })), Some("List files".into()));
        assert_eq!(summary(&json!({ "command": "ls", "description": "" })), None);
        assert_eq!(summary(&json!({ "file_path": "a.rs" })), None);
        assert_eq!(project_name(r"C:\Users\joni\Projekte\rice\"), "rice");
        assert_eq!(project_name("/home/joni/rice"), "rice");
    }

    #[test]
    fn reads_claude_payloads() {
        let permission = json!({
            "session_id": "s1", "cwd": r"C:\rice", "hook_event_name": "PermissionRequest",
            "tool_name": "Bash", "tool_input": { "command": "ls" }, "permission_mode": "default"
        });
        assert_eq!(
            claude_message(&permission),
            Some(PipeMessage::Permission {
                v: PROTOCOL, agent: "claude".into(), session: "s1".into(), cwd: r"C:\rice".into(), tool: "Bash".into(),
                input: json!({ "command": "ls" }),
            })
        );
        let stop = json!({ "hook_event_name": "Stop", "session_id": "s1", "cwd": "C:\\rice", "last_assistant_message": "Fertig." });
        assert!(matches!(claude_message(&stop), Some(PipeMessage::Status { state: AgentStatusState::Done, message: Some(m), .. }) if m == "Fertig."));
        let idle = json!({ "hook_event_name": "Notification", "notification_type": "idle_prompt", "message": "waiting" });
        assert!(matches!(claude_message(&idle), Some(PipeMessage::Status { state: AgentStatusState::Waiting, .. })));
        assert_eq!(claude_message(&json!({ "hook_event_name": "Notification", "notification_type": "permission_prompt" })), None);
        assert_eq!(claude_message(&json!({ "hook_event_name": "PreToolUse" })), None);
    }

    #[test]
    fn answers_claude() {
        let allow = claude_answer(Decision::Allow).unwrap();
        assert_eq!(allow["hookSpecificOutput"]["hookEventName"], "PermissionRequest");
        assert_eq!(allow["hookSpecificOutput"]["decision"]["behavior"], "allow");
        assert_eq!(claude_answer(Decision::Deny).unwrap()["hookSpecificOutput"]["decision"]["behavior"], "deny");
        assert_eq!(claude_answer(Decision::Terminal), None);
        let message: PipeMessage = serde_json::from_str(
            r#"{"type":"status","v":1,"agent":"claude","session":"","cwd":"","state":"waiting","message":null}"#,
        )
        .unwrap();
        assert!(matches!(message, PipeMessage::Status { .. }));
        assert_eq!(serde_json::to_string(&PipeAnswer { decision: Decision::Terminal }).unwrap(), r#"{"decision":"terminal"}"#);
    }

    #[test]
    fn edits_claude_settings_around_existing_hooks() {
        let original = json!({
            "model": "opus",
            "hooks": { "SessionStart": [{ "hooks": [{ "type": "command", "command": "echo hi" }] }] }
        });
        let mut settings = original.clone();
        assert!(!claude_connected(&settings));
        claude_connect(&mut settings, r"C:\rice\rice.exe").unwrap();
        assert!(claude_connected(&settings));
        let permission = &settings["hooks"]["PermissionRequest"][0];
        assert_eq!(permission["matcher"], "*");
        assert_eq!(permission["hooks"][0]["args"], json!(["--agent-hook", "claude"]));
        assert_eq!(permission["hooks"][0]["command"], r"C:\rice\rice.exe");
        assert_eq!(settings["hooks"]["Stop"][0].get("matcher"), None);
        // Connecting again (e.g. from a new install path) replaces, never duplicates.
        claude_connect(&mut settings, r"D:\rice.exe").unwrap();
        assert_eq!(settings["hooks"]["PermissionRequest"].as_array().unwrap().len(), 1);
        assert!(claude_disconnect(&mut settings));
        assert_eq!(settings, original);
        assert!(!claude_disconnect(&mut settings));

        // A user hook in the same group survives; the empty hooks object goes.
        let mut shared = json!({ "hooks": {} });
        claude_connect(&mut shared, "rice.exe").unwrap();
        shared["hooks"]["Stop"][0]["hooks"].as_array_mut().unwrap().push(json!({ "type": "command", "command": "beep" }));
        claude_disconnect(&mut shared);
        assert_eq!(shared, json!({ "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "beep" }] }] } }));
        let mut empty = json!({});
        claude_connect(&mut empty, "rice.exe").unwrap();
        claude_disconnect(&mut empty);
        assert_eq!(empty, json!({}));

        assert!(claude_connect(&mut json!([1, 2]), "rice.exe").is_err());
        assert!(claude_connect(&mut json!({ "hooks": [] }), "rice.exe").is_err());
    }
}
