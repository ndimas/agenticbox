//! AgenticBox microkernel.
//!
//! The harness is a small core (`Harness`) that owns a `PluginRegistry`.
//! Everything else — filesystem access, network access, exec, audit — is a
//! plugin implementing the [`Plugin`] trait. The core knows nothing about
//! any specific capability; it routes, sequences, and reports.
//!
//! Plugin kinds:
//! - [`ToolPlugin`]: handles an agent tool call (`read_file`, `exec`, ...).
//! - [`HookPlugin`]: observes lifecycle events (session start/end, decision made).
//!
//! Registration order matters: tools are matched by name (first wins), hooks
//! run in registration order.

use anyhow::Result;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Outcome of a tool plugin handling a call.
#[derive(Debug, Clone)]
pub struct ToolOutcome {
    /// Whether the action was permitted and (attempted) executed.
    pub allowed: bool,
    /// Human-readable reason (shown in the action log / audit trail).
    pub reason: String,
    /// Output fed back to the model on success.
    pub output: String,
}

impl ToolOutcome {
    pub fn allowed(reason: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            allowed: true,
            reason: reason.into(),
            output: output.into(),
        }
    }
    pub fn blocked(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: reason.into(),
            output: String::new(),
        }
    }
}

/// A single agent tool call, already parsed from JSON arguments.
#[derive(Debug, Clone)]
pub struct ToolCall<'a> {
    pub name: &'a str,
    pub args: &'a serde_json::Value,
}

/// Lifecycle events hooks can observe.
#[derive(Debug, Clone)]
pub enum HarnessEvent<'a> {
    SessionStart {
        agent: &'a str,
    },
    SessionEnd {
        agent: &'a str,
        allowed: u32,
        blocked: u32,
    },
    DecisionMade {
        tool: &'a str,
        allowed: bool,
        reason: &'a str,
    },
}

/// A plugin that handles one or more agent tools.
pub trait ToolPlugin: Send + Sync {
    /// Tool names this plugin handles (used for schema generation and routing).
    fn tool_names(&self) -> Vec<String>;

    /// OpenAI-style JSON schema for each tool this plugin provides.
    fn schemas(&self) -> Vec<serde_json::Value>;

    /// Handle a tool call. `ctx` gives read access to shared harness config.
    fn handle(&self, call: ToolCall<'_>, ctx: &HarnessContext) -> Result<ToolOutcome>;
}

/// A plugin that observes lifecycle events (audit, metrics, ...).
pub trait HookPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn on_event(&self, event: HarnessEvent<'_>);
}

/// Read-only context handed to tool plugins.
#[derive(Debug, Clone, Default)]
pub struct HarnessContext {
    /// Workspace root the agent is allowed to touch.
    pub workspace: std::path::PathBuf,
    /// Network allowlist (domains).
    pub network_allowlist: Vec<String>,
    /// Arbitrary string config from the manifest/profile.
    pub config: BTreeMap<String, String>,
}

type Tool = Arc<dyn ToolPlugin>;
type Hook = Arc<dyn HookPlugin>;

/// The microkernel. Small on purpose: it routes tool calls to plugins and
/// broadcasts events to hooks. All capability logic lives in plugins.
#[derive(Default)]
pub struct Harness {
    tools: BTreeMap<String, Tool>,
    hooks: Vec<Hook>,
    ctx: HarnessContext,
}

impl Harness {
    pub fn new(ctx: HarnessContext) -> Self {
        Self {
            tools: BTreeMap::new(),
            hooks: Vec::new(),
            ctx,
        }
    }

    /// Shared read-only context (also handed to plugins).
    pub fn context(&self) -> &HarnessContext {
        &self.ctx
    }

    /// Register a tool plugin. First registration of a name wins.
    pub fn register_tool(&mut self, plugin: Arc<dyn ToolPlugin>) {
        for name in plugin.tool_names() {
            self.tools.entry(name).or_insert(plugin.clone());
        }
    }

    /// Register a hook plugin (runs in registration order).
    pub fn register_hook(&mut self, plugin: Arc<dyn HookPlugin>) {
        self.hooks.push(plugin);
    }

    /// OpenAI-style tool schema list, aggregated from all tool plugins.
    pub fn tool_schemas(&self) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for plugin in self.tools.values() {
            for schema in plugin.schemas() {
                // dedupe by function name
                let name = schema
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string();
                if seen.insert(name) {
                    out.push(schema);
                }
            }
        }
        out
    }

    /// Route one tool call. Emits `DecisionMade` afterwards.
    ///
    /// Unknown tools are blocked by the core (fail-closed), so plugins can
    /// never be bypassed by a hallucinated tool name.
    pub fn dispatch(&self, call: ToolCall<'_>) -> Result<ToolOutcome> {
        let outcome = match self.tools.get(call.name) {
            Some(plugin) => plugin.handle(call.clone(), &self.ctx)?,
            None => ToolOutcome::blocked(format!("unknown tool: {}", call.name)),
        };
        self.emit(HarnessEvent::DecisionMade {
            tool: call.name,
            allowed: outcome.allowed,
            reason: &outcome.reason,
        });
        Ok(outcome)
    }

    /// Broadcast an event to all hooks.
    pub fn emit(&self, event: HarnessEvent<'_>) {
        for hook in &self.hooks {
            hook.on_event(event.clone());
        }
    }

    /// Number of distinct registered tools.
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct EchoPlugin {
        calls: AtomicU32,
    }

    impl ToolPlugin for EchoPlugin {
        fn tool_names(&self) -> Vec<String> {
            vec!["echo".into()]
        }
        fn schemas(&self) -> Vec<serde_json::Value> {
            vec![serde_json::json!({
                "type": "function",
                "function": {
                    "name": "echo",
                    "description": "Echo the input",
                    "parameters": {"type": "object", "properties": {"text": {"type": "string"}}}
                }
            })]
        }
        fn handle(&self, _call: ToolCall<'_>, _ctx: &HarnessContext) -> Result<ToolOutcome> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(ToolOutcome::allowed("echoed", "ok"))
        }
    }

    struct EventLog(std::sync::Mutex<Vec<String>>);

    impl HookPlugin for EventLog {
        fn name(&self) -> &str {
            "event-log"
        }
        fn on_event(&self, event: HarnessEvent<'_>) {
            let mut log = self.0.lock().unwrap();
            match event {
                HarnessEvent::DecisionMade { tool, allowed, .. } => {
                    log.push(format!("decision:{tool}:{allowed}"));
                }
                HarnessEvent::SessionStart { agent, .. } => {
                    log.push(format!("start:{agent}"));
                }
                HarnessEvent::SessionEnd { agent, .. } => {
                    log.push(format!("end:{agent}"));
                }
            }
        }
    }

    #[test]
    fn dispatch_routes_to_plugin() {
        let mut h = Harness::new(HarnessContext::default());
        h.register_tool(Arc::new(EchoPlugin {
            calls: AtomicU32::new(0),
        }));
        let args = serde_json::json!({"text": "hi"});
        let out = h
            .dispatch(ToolCall {
                name: "echo",
                args: &args,
            })
            .unwrap();
        assert!(out.allowed);
        assert_eq!(h.tool_count(), 1);
    }

    #[test]
    fn unknown_tool_fails_closed() {
        let h = Harness::new(HarnessContext::default());
        let args = serde_json::json!({});
        let out = h
            .dispatch(ToolCall {
                name: "nope",
                args: &args,
            })
            .unwrap();
        assert!(!out.allowed);
        assert!(out.reason.contains("unknown tool"));
    }

    #[test]
    fn hooks_see_decisions_and_lifecycle() {
        let log = Arc::new(EventLog(std::sync::Mutex::new(Vec::new())));
        let mut h = Harness::new(HarnessContext::default());
        h.register_tool(Arc::new(EchoPlugin {
            calls: AtomicU32::new(0),
        }));
        h.register_hook(log.clone());

        h.emit(HarnessEvent::SessionStart {
            agent: "test-agent",
        });
        let args = serde_json::json!({});
        h.dispatch(ToolCall {
            name: "echo",
            args: &args,
        })
        .unwrap();
        h.emit(HarnessEvent::SessionEnd {
            agent: "test-agent",
            allowed: 1,
            blocked: 0,
        });

        let got = log.0.lock().unwrap().clone();
        assert_eq!(
            got,
            vec!["start:test-agent", "decision:echo:true", "end:test-agent"]
        );
    }

    #[test]
    fn schemas_aggregate_and_dedupe() {
        let mut h = Harness::new(HarnessContext::default());
        h.register_tool(Arc::new(EchoPlugin {
            calls: AtomicU32::new(0),
        }));
        h.register_tool(Arc::new(EchoPlugin {
            calls: AtomicU32::new(0),
        }));
        let schemas = h.tool_schemas();
        assert_eq!(schemas.len(), 1, "duplicate plugin schemas must dedupe");
    }
}
