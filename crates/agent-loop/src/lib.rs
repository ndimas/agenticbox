use anyhow::{Context, Result};
use console::style;
use harness_core::{Harness, HarnessContext};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// ─── Public types ─────────────────────────────────────────────

/// Compute the trust score delta from a session's decision history.
///
/// Deterministic, no LLM. Scoring rules:
///   Clean session end (0 denies)      → +1
///   Each `fs:write` Deny               → -2
///   Each `network:outbound` Deny       → -5 (exfiltration attempt)
///   Each `terminal:exec` Deny          → -3
///   Each other Deny                    → -1
///
/// The delta is clamped to a minimum of -20 per session (one bad session
/// should not irreversibly tank an otherwise good identity).
pub fn compute_trust_delta(history: &[DecisionLog]) -> i32 {
    let denies: Vec<&DecisionLog> = history.iter().filter(|d| !d.allowed).collect();

    if denies.is_empty() {
        return 1; // clean session
    }

    let mut delta: i32 = 0;
    for d in &denies {
        let penalty = match d.tool.as_str() {
            "network:outbound" | "http_request" => -5,
            "terminal:exec" | "exec" => -3,
            "fs:write" | "write_file" => -2,
            _ => -1,
        };
        delta += penalty;
    }

    delta.max(-20) // clamp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLoopConfig {
    pub api_base: String,
    pub model: String,
    pub workspace: PathBuf,
    pub network_allowlist: Vec<String>,
    pub max_iterations: usize,
    pub system_prompt: String,
    pub user_task: String,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            api_base: "http://localhost:1234/v1".into(),
            model: "huihui-qwen3.6-35b-a3b-claude-4.7-opus-abliterated-mtp@q5_k".into(),
            workspace: PathBuf::from("./workspace"),
            network_allowlist: vec!["api.github.com".into(), "registry.npmjs.org".into()],
            max_iterations: 15,
            system_prompt: String::new(),
            user_task: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLoopResult {
    pub allowed: u32,
    pub blocked: u32,
    pub history: Vec<DecisionLog>,
    pub final_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionLog {
    pub timestamp: String,
    pub tool: String,
    pub args: String,
    pub allowed: bool,
    pub reason: String,
    pub agent_message: Option<String>,
}

// ─── OpenAI-compatible API types ──────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    tools: Vec<ToolDefinition>,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct ToolDefinition {
    #[serde(rename = "type")]
    def_type: String,
    function: ToolFunction,
}

#[derive(Debug, Serialize)]
struct ToolFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

// ─── Tool definitions (OpenAI function-calling schema) ────────

fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            def_type: "function".into(),
            function: ToolFunction {
                name: "read_file".into(),
                description: "Read the contents of a file. The path must be within the allowed workspace directory.".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Absolute or relative path to the file" }
                    },
                    "required": ["path"]
                }),
            },
        },
        ToolDefinition {
            def_type: "function".into(),
            function: ToolFunction {
                name: "write_file".into(),
                description: "Write content to a file. The path must be within the allowed workspace directory.".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path to write to" },
                        "content": { "type": "string", "description": "Content to write" }
                    },
                    "required": ["path", "content"]
                }),
            },
        },
        ToolDefinition {
            def_type: "function".into(),
            function: ToolFunction {
                name: "http_request".into(),
                description: "Make an HTTP request to a URL. Only allowlisted domains are permitted.".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "Full URL to request" },
                        "method": { "type": "string", "description": "HTTP method (GET, POST, etc.)" }
                    },
                    "required": ["url", "method"]
                }),
            },
        },
        ToolDefinition {
            def_type: "function".into(),
            function: ToolFunction {
                name: "exec".into(),
                description: "Execute a shell command in the sandbox.".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": { "type": "string", "description": "Command to execute" }
                    },
                    "required": ["command"]
                }),
            },
        },
    ]
}

// ─── Helpers ──────────────────────────────────────────────────

fn ts() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!(
        "{:02}:{:02}:{:02}",
        (secs % 86400) / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

/// Extract the most relevant target string from tool args (path, url, or command)
fn extract_target(tool: &str, args: &serde_json::Value) -> String {
    match tool {
        "read_file" | "write_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            // Show just the filename for readability
            std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path)
                .to_string()
        }
        "http_request" => args
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string(),
        "exec" => {
            let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("?");
            // Show first 45 chars of command
            cmd.chars().take(45).collect()
        }
        _ => "?".to_string(),
    }
}

/// Print one clean line per action: entire line green for ALLOWED, red for BLOCKED
fn print_action(tool: &str, target: &str, allowed: bool, reason: &str) {
    let target_display: String = target.chars().take(45).collect();
    let icon = if allowed { "✓" } else { "✗" };
    let status = if allowed { "ALLOWED" } else { "BLOCKED" };

    if allowed {
        // Entire line green
        let line = format!("  {} {:<12} {:<47} {}", icon, tool, target_display, status);
        println!("{}", style(line).green());
    } else {
        // Entire line red
        let line = format!("  {} {:<12} {:<47} {}", icon, tool, target_display, status);
        println!("{}", style(line).red());
        // Reason on next line, dim
        println!("    {} {}", style("→").dim(), style(reason).dim());
    }

    // Pace the output so it's readable
    std::thread::sleep(std::time::Duration::from_millis(400));
}

// ─── The agent loop ───────────────────────────────────────────

pub async fn run_agent_loop(config: AgentLoopConfig) -> Result<AgentLoopResult> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    // Microkernel: register builtin tool plugins. All capability logic
    // (fs containment, network allowlist, exec policy) lives in the plugins;
    // the loop itself only routes and records.
    let mut harness = Harness::new(HarnessContext {
        workspace: config.workspace.clone(),
        network_allowlist: config.network_allowlist.clone(),
        config: Default::default(),
    });
    harness.register_tool(std::sync::Arc::new(plugin_builtin_tools::FsPlugin));
    harness.register_tool(std::sync::Arc::new(plugin_builtin_tools::NetworkPlugin));
    harness.register_tool(std::sync::Arc::new(plugin_builtin_tools::ExecPlugin));
    let harness = harness; // freeze (interior mutability not needed)

    let mut allowed: u32 = 0;
    let mut blocked: u32 = 0;
    let mut pending_exec: u32 = 0; // batch consecutive allowed exec calls
    let mut history: Vec<DecisionLog> = Vec::new();
    let mut final_message = String::new();

    // Build initial messages
    let mut messages = vec![
        ChatMessage {
            role: "system".into(),
            content: Some(config.system_prompt.clone()),
            tool_calls: None,
            tool_call_id: None,
        },
        ChatMessage {
            role: "user".into(),
            content: Some(config.user_task.clone()),
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    let endpoint = format!("{}/chat/completions", config.api_base.trim_end_matches('/'));

    for _iteration in 0..config.max_iterations {
        // Call the LLM
        let req_body = ChatRequest {
            model: config.model.clone(),
            messages: messages.clone(),
            tools: tool_definitions(),
            temperature: 0.7,
        };

        let resp = client.post(&endpoint).json(&req_body).send().await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                anyhow::bail!(
                    "Failed to reach LLM at {}: {}. Is LM Studio or another OpenAI-compatible server running?",
                    endpoint, e
                );
            }
        };

        let status = resp.status();
        let body = resp
            .text()
            .await
            .context("Failed to read LLM response body")?;

        if !status.is_success() {
            // Extract OpenAI-style error message: {"error": {"message": "..."}}
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(msg) = val
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                {
                    anyhow::bail!("LLM API error (HTTP {}): {}", status, msg);
                }
            }
            anyhow::bail!(
                "LLM API returned HTTP {}: {}",
                status,
                &body[..body.len().min(500)]
            );
        }

        let chat_resp: ChatResponse = serde_json::from_str(&body).with_context(|| {
            format!(
                "Failed to parse LLM response (HTTP {}). Body (first 300 chars): {}",
                status,
                &body[..body.len().min(300)]
            )
        })?;

        let choice = chat_resp
            .choices
            .into_iter()
            .next()
            .context("No choices in LLM response")?;

        let assistant_msg = choice.message;

        // Capture final message silently (don't print — keeps output clean)
        if let Some(content) = &assistant_msg.content {
            if !content.is_empty() {
                final_message = content.clone();
            }
        }

        let tool_calls = match &assistant_msg.tool_calls {
            Some(calls) if !calls.is_empty() => calls.clone(),
            _ => {
                // No tool calls — agent is done
                break;
            }
        };

        // Add assistant message to history
        messages.push(assistant_msg.clone());

        // Execute each tool call through the guards
        for tc in &tool_calls {
            let args: serde_json::Value =
                serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::json!({}));

            let tool_name = &tc.function.name;
            let args_str = serde_json::to_string(&args).unwrap_or_default();

            let (is_allowed, reason, _output) = {
                let outcome = harness.dispatch(harness_core::ToolCall {
                    name: tool_name,
                    args: &args,
                })?;
                (outcome.allowed, outcome.reason, outcome.output)
            };

            let target = extract_target(tool_name, &args);

            if is_allowed {
                allowed += 1;
            } else {
                blocked += 1;
            }

            // Batch consecutive allowed exec calls into one line
            if tool_name == "exec" && is_allowed {
                pending_exec += 1;
            } else {
                // Flush pending exec batch first
                if pending_exec > 0 {
                    let noun = if pending_exec == 1 {
                        "command"
                    } else {
                        "commands"
                    };
                    let line = format!(
                        "  {} {:<12} {:<47} {}",
                        "✓",
                        "exec",
                        format!("{} shell {}", pending_exec, noun),
                        "ALLOWED"
                    );
                    println!("{}", style(line).green());
                    std::thread::sleep(std::time::Duration::from_millis(400));
                    pending_exec = 0;
                }
                // Print this action normally
                print_action(tool_name, &target, is_allowed, &reason);
            }

            history.push(DecisionLog {
                timestamp: ts(),
                tool: tool_name.clone(),
                args: args_str,
                allowed: is_allowed,
                reason: reason.clone(),
                agent_message: assistant_msg.content.clone(),
            });

            // Feed result back to LLM
            let tool_result = if is_allowed {
                _output.clone()
            } else {
                format!("BLOCKED: {reason}")
            };

            messages.push(ChatMessage {
                role: "tool".into(),
                content: Some(tool_result),
                tool_calls: None,
                tool_call_id: Some(tc.id.clone()),
            });
        }
    }

    // Flush any remaining batched exec calls
    if pending_exec > 0 {
        let noun = if pending_exec == 1 {
            "command"
        } else {
            "commands"
        };
        let line = format!(
            "  {} {:<12} {:<47} {}",
            "✓",
            "exec",
            format!("{} shell {}", pending_exec, noun),
            "ALLOWED"
        );
        println!("{}", style(line).green());
    }

    Ok(AgentLoopResult {
        allowed,
        blocked,
        history,
        final_message,
    })
}

// ─── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_decision(allowed: bool, tool: &str) -> DecisionLog {
        DecisionLog {
            timestamp: "2025-01-01T00:00:00Z".into(),
            tool: tool.into(),
            args: "test".into(),
            allowed,
            reason: if allowed { "allowed" } else { "blocked" }.into(),
            agent_message: None,
        }
    }

    #[test]
    fn clean_session_returns_plus_one() {
        let history = vec![
            make_decision(true, "fs:read"),
            make_decision(true, "network:outbound"),
            make_decision(true, "terminal:exec"),
        ];
        assert_eq!(compute_trust_delta(&history), 1);
    }

    #[test]
    fn empty_history_returns_plus_one() {
        let history: Vec<DecisionLog> = vec![];
        assert_eq!(compute_trust_delta(&history), 1);
    }

    #[test]
    fn single_network_deny_is_minus_five() {
        let history = vec![make_decision(false, "network:outbound")];
        assert_eq!(compute_trust_delta(&history), -5);
    }

    #[test]
    fn single_fs_write_deny_is_minus_two() {
        let history = vec![make_decision(false, "fs:write")];
        assert_eq!(compute_trust_delta(&history), -2);
    }

    #[test]
    fn single_terminal_deny_is_minus_three() {
        let history = vec![make_decision(false, "terminal:exec")];
        assert_eq!(compute_trust_delta(&history), -3);
    }

    #[test]
    fn unknown_tool_deny_is_minus_one() {
        let history = vec![make_decision(false, "browser:navigate")];
        assert_eq!(compute_trust_delta(&history), -1);
    }

    #[test]
    fn http_request_counts_as_network() {
        let history = vec![make_decision(false, "http_request")];
        assert_eq!(compute_trust_delta(&history), -5);
    }

    #[test]
    fn exec_counts_as_terminal() {
        let history = vec![make_decision(false, "exec")];
        assert_eq!(compute_trust_delta(&history), -3);
    }

    #[test]
    fn write_file_counts_as_fs_write() {
        let history = vec![make_decision(false, "write_file")];
        assert_eq!(compute_trust_delta(&history), -2);
    }

    #[test]
    fn multiple_denies_accumulate() {
        let history = vec![
            make_decision(false, "network:outbound"), // -5
            make_decision(false, "fs:write"),         // -2
            make_decision(false, "terminal:exec"),    // -3
            make_decision(false, "browser:navigate"), // -1
        ];
        assert_eq!(compute_trust_delta(&history), -11);
    }

    #[test]
    fn clamp_at_minus_twenty() {
        let history: Vec<DecisionLog> = (0..10)
            .map(|_| make_decision(false, "network:outbound"))
            .collect(); // 10 * -5 = -50, clamped to -20
        assert_eq!(compute_trust_delta(&history), -20);
    }

    #[test]
    fn mixed_allowed_and_denied() {
        let history = vec![
            make_decision(true, "fs:read"),
            make_decision(false, "network:outbound"), // -5
            make_decision(true, "terminal:exec"),
            make_decision(false, "fs:write"), // -2
        ];
        assert_eq!(compute_trust_delta(&history), -7);
    }
}
