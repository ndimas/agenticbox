//! Builtin tool plugins for AgenticBox: filesystem, network, exec.
//!
//! Each plugin wraps the existing guard crates (fs-guard, network-control,
//! policy-engine) behind the harness-core `ToolPlugin` trait. Behavior is
//! identical to the pre-plugin agent-loop implementations — this is an
//! extraction, not a rewrite.

use anyhow::Result;
use harness_core::{HarnessContext, ToolCall, ToolOutcome, ToolPlugin};
use policy_engine::PolicyEngine;
use shared_types::{FsPermission, NetworkPolicy, PermissionSet};

// ─── Filesystem tools ─────────────────────────────────────────

/// `read_file` / `write_file`, guarded by `FsGuard` against the workspace root.
pub struct FsPlugin;

pub fn schema(name: &str, description: &str, properties: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": {
                "type": "object",
                "properties": properties,
            }
        }
    })
}

impl ToolPlugin for FsPlugin {
    fn tool_names(&self) -> Vec<String> {
        vec!["read_file".into(), "write_file".into()]
    }

    fn schemas(&self) -> Vec<serde_json::Value> {
        vec![
            schema(
                "read_file",
                "Read the contents of a file. The path must be within the allowed workspace directory.",
                serde_json::json!({"path": {"type": "string", "description": "Absolute or relative path to the file"}}),
            ),
            schema(
                "write_file",
                "Write content to a file. The path must be within the allowed workspace directory.",
                serde_json::json!({
                    "path": {"type": "string", "description": "Path to write to"},
                    "content": {"type": "string", "description": "Content to write"}
                }),
            ),
        ]
    }

    fn handle(&self, call: ToolCall<'_>, ctx: &HarnessContext) -> Result<ToolOutcome> {
        let guard = fs_guard::FsGuard::new(vec![ctx.workspace.clone()]);
        match call.name {
            "read_file" => {
                let path = call.args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                // Note: FsGuard::resolve enforces the workspace root; the
                // actual read error is surfaced through `Escaped`-style
                // mapping below (resolve errors keep their own variants).
                match guard.resolve(path) {
                    Ok(resolved) => match std::fs::read_to_string(&resolved) {
                        Ok(content) => Ok(ToolOutcome::allowed("within allowed roots", content)),
                        Err(e) => Ok(ToolOutcome::blocked(format!("read error: {e}"))),
                    },
                    Err(e) => Ok(ToolOutcome::blocked(format!("filesystem: {e}"))),
                }
            }
            "write_file" => {
                let path = call.args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                let content = call
                    .args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                match guard.resolve(path) {
                    Ok(resolved) => match std::fs::write(&resolved, content) {
                        Ok(()) => Ok(ToolOutcome::allowed(
                            "within allowed roots",
                            "File written successfully",
                        )),
                        Err(e) => Ok(ToolOutcome::blocked(format!("write error: {e}"))),
                    },
                    Err(e) => Ok(ToolOutcome::blocked(format!("filesystem: {e}"))),
                }
            }
            other => Ok(ToolOutcome::blocked(format!(
                "fs plugin cannot handle {other}"
            ))),
        }
    }
}

// ─── Network tool ─────────────────────────────────────────────

/// `http_request`, domain-checked by `NetworkGuard` against the allowlist.
pub struct NetworkPlugin;

impl ToolPlugin for NetworkPlugin {
    fn tool_names(&self) -> Vec<String> {
        vec!["http_request".into()]
    }

    fn schemas(&self) -> Vec<serde_json::Value> {
        vec![schema(
            "http_request",
            "Make an HTTP request to a URL. Only allowlisted domains are permitted.",
            serde_json::json!({
                "url": {"type": "string", "description": "Full URL to request"},
                "method": {"type": "string", "description": "HTTP method (GET, POST, etc.)"}
            }),
        )]
    }

    fn handle(&self, call: ToolCall<'_>, ctx: &HarnessContext) -> Result<ToolOutcome> {
        let url = call.args.get("url").and_then(|v| v.as_str()).unwrap_or("");
        let _method = call
            .args
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET");
        let guard = network_control::NetworkGuard::new(NetworkPolicy::Allowlist(
            ctx.network_allowlist.clone(),
        ));
        match guard.check(url) {
            Ok(()) => Ok(ToolOutcome::allowed(
                "domain in allowlist",
                format!("HTTP 200 OK (simulated — {url} is allowlisted)"),
            )),
            Err(e) => Ok(ToolOutcome::blocked(format!("network: {e}"))),
        }
    }
}

// ─── Exec tool ────────────────────────────────────────────────

/// `exec`, gated by the `PolicyEngine` terminal permission.
pub struct ExecPlugin;

impl ExecPlugin {
    fn evaluate(&self, command: &str) -> policy_engine::PolicyDecision {
        let engine = PolicyEngine::new();
        let req = policy_engine::PolicyRequest {
            action: "terminal:exec".into(),
            resource: command.into(),
            permissions: PermissionSet {
                terminal: true,
                filesystem: FsPermission::ReadWrite,
                browser: false,
                network: NetworkPolicy::Allowlist(vec![]),
            },
        };
        engine.evaluate(req)
    }
}

impl ToolPlugin for ExecPlugin {
    fn tool_names(&self) -> Vec<String> {
        vec!["exec".into()]
    }

    fn schemas(&self) -> Vec<serde_json::Value> {
        vec![schema(
            "exec",
            "Execute a shell command in the sandbox.",
            serde_json::json!({"command": {"type": "string", "description": "Command to execute"}}),
        )]
    }

    fn handle(&self, call: ToolCall<'_>, _ctx: &HarnessContext) -> Result<ToolOutcome> {
        let command = call
            .args
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        match self.evaluate(command) {
            policy_engine::PolicyDecision::Allow => Ok(ToolOutcome::allowed(
                "terminal exec permitted",
                format!("executed: {command}"),
            )),
            policy_engine::PolicyDecision::Deny(reason) => {
                Ok(ToolOutcome::blocked(format!("terminal: {reason}")))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use harness_core::Harness;

    fn ctx() -> HarnessContext {
        // Canonicalize to match FsGuard's internal canonicalization (Windows
        // extended-length `\\?\` prefixes otherwise break starts_with).
        let dir =
            std::env::temp_dir().join(format!("agenticbox-plugin-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let dir = dir.canonicalize().unwrap_or(dir);
        HarnessContext {
            workspace: dir,
            network_allowlist: vec!["api.github.com".into()],
            config: Default::default(),
        }
    }

    #[test]
    fn read_write_roundtrip_inside_workspace() {
        let c = ctx();
        let mut h = Harness::new(c.clone());
        h.register_tool(std::sync::Arc::new(FsPlugin));

        let args = serde_json::json!({"path": "hello.txt", "content": "world"});
        let out = h
            .dispatch(ToolCall {
                name: "write_file",
                args: &args,
            })
            .unwrap();
        assert!(out.allowed, "write blocked: {}", out.reason);

        let args = serde_json::json!({"path": "hello.txt"});
        let out = h
            .dispatch(ToolCall {
                name: "read_file",
                args: &args,
            })
            .unwrap();
        assert!(out.allowed);
        assert_eq!(out.output, "world");
    }

    #[test]
    fn read_outside_workspace_blocked() {
        let c = ctx();
        let mut h = Harness::new(c);
        h.register_tool(std::sync::Arc::new(FsPlugin));
        let args = serde_json::json!({"path": "C:/Windows/win.ini"});
        let out = h
            .dispatch(ToolCall {
                name: "read_file",
                args: &args,
            })
            .unwrap();
        assert!(!out.allowed);
    }

    #[test]
    fn network_allowlist_enforced() {
        let c = ctx();
        let mut h = Harness::new(c);
        h.register_tool(std::sync::Arc::new(NetworkPlugin));

        let ok = serde_json::json!({"url": "https://api.github.com/x", "method": "GET"});
        assert!(
            h.dispatch(ToolCall {
                name: "http_request",
                args: &ok
            })
            .unwrap()
            .allowed
        );

        let bad = serde_json::json!({"url": "https://evil.example.com/x", "method": "GET"});
        assert!(
            !h.dispatch(ToolCall {
                name: "http_request",
                args: &bad
            })
            .unwrap()
            .allowed
        );
    }

    #[test]
    fn exec_permitted_by_policy() {
        let c = ctx();
        let mut h = Harness::new(c);
        h.register_tool(std::sync::Arc::new(ExecPlugin));
        let args = serde_json::json!({"command": "echo hi"});
        let out = h
            .dispatch(ToolCall {
                name: "exec",
                args: &args,
            })
            .unwrap();
        assert!(out.allowed, "exec blocked: {}", out.reason);
    }

    #[test]
    fn all_builtin_schemas_registered() {
        let c = ctx();
        let mut h = Harness::new(c);
        h.register_tool(std::sync::Arc::new(FsPlugin));
        h.register_tool(std::sync::Arc::new(NetworkPlugin));
        h.register_tool(std::sync::Arc::new(ExecPlugin));
        let names: Vec<String> = h
            .tool_schemas()
            .iter()
            .filter_map(|s| s.get("function")?.get("name")?.as_str().map(String::from))
            .collect();
        assert_eq!(
            names,
            vec!["exec", "http_request", "read_file", "write_file"] // BTreeMap: alphabetical
        );
    }
}
