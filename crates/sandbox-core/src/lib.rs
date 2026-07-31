mod runtime;

pub use runtime::ContainerRuntime;

use bollard::container::{
    Config, CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions,
    StartContainerOptions, StopContainerOptions, WaitContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::models::{HostConfig, MountTypeEnum};
use bollard::Docker;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub use bollard::errors::Error as ContainerError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub image: String,
    pub cmd: Vec<String>,
    pub env: HashMap<String, String>,
    pub mounts: Vec<SandboxMount>,
    pub resources: SandboxResources,
    pub network_mode: String,
    pub working_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxMount {
    pub source: String,
    pub target: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SandboxResources {
    pub memory_mb: Option<i64>,
    pub cpu_count: Option<i64>,
}

pub struct SandboxHandle {
    pub id: String,
    client: Docker,
}

impl SandboxHandle {
    pub fn new(id: String, client: Docker) -> Self {
        Self { id, client }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        self.client
            .start_container(&self.id, None::<StartContainerOptions<String>>)
            .await?;
        Ok(())
    }

    pub async fn stop(&self, timeout: Option<i64>) -> anyhow::Result<()> {
        let opts = timeout.map(|t| StopContainerOptions { t });
        self.client.stop_container(&self.id, opts).await?;
        Ok(())
    }

    pub async fn remove(&self, force: bool) -> anyhow::Result<()> {
        let opts = RemoveContainerOptions {
            force,
            ..Default::default()
        };
        self.client.remove_container(&self.id, Some(opts)).await?;
        Ok(())
    }

    /// Stream logs (stdout + stderr) from the container in real-time.
    pub async fn stream_logs<F>(&self, on_line: F) -> anyhow::Result<()>
    where
        F: Fn(LogLine),
    {
        let options = Some(LogsOptions::<String> {
            follow: true,
            stdout: true,
            stderr: true,
            timestamps: false,
            ..Default::default()
        });

        let mut log_stream = self.client.logs(&self.id, options);
        while let Some(msg) = log_stream.next().await {
            match msg {
                Ok(LogOutput::StdOut { message }) => {
                    let text = String::from_utf8_lossy(&message);
                    for line in text.lines() {
                        on_line(LogLine::Stdout(line.to_string()));
                    }
                }
                Ok(LogOutput::StdErr { message }) => {
                    let text = String::from_utf8_lossy(&message);
                    for line in text.lines() {
                        on_line(LogLine::Stderr(line.to_string()));
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::debug!("Log stream error: {:?}", e);
                    break;
                }
            }
        }
        Ok(())
    }

    /// Wait for the container to finish and return the exit code.
    pub async fn wait(&self) -> anyhow::Result<i64> {
        let options = Some(WaitContainerOptions {
            condition: "not-running",
        });

        let mut wait_stream = self.client.wait_container(&self.id, options);
        if let Some(msg) = wait_stream.next().await {
            match msg {
                Ok(resp) => Ok(resp.status_code),
                Err(e) => Err(anyhow::anyhow!("Wait failed: {:?}", e)),
            }
        } else {
            Ok(0)
        }
    }

    /// Exec into the container with an interactive stdin/stdout pipe.
    ///
    /// This is the ACP transport: the host can write to the agent's stdin
    /// and read from its stdout. The `on_output` callback receives each
    /// chunk of stdout/stderr as it arrives.
    ///
    /// The returned `ExecPipe` lets the caller write to the container's stdin.
    /// Drop it to close the pipe.
    pub async fn exec_interactive(
        &self,
        cmd: Vec<String>,
        tty: bool,
        on_output: impl Fn(ExecOutput) + Send + 'static,
    ) -> anyhow::Result<ExecPipe> {
        let exec = self
            .client
            .create_exec(
                &self.id,
                CreateExecOptions {
                    cmd: Some(cmd),
                    attach_stdout: Some(true),
                    attach_stderr: Some(!tty),
                    attach_stdin: Some(true),
                    tty: Some(tty),
                    ..Default::default()
                },
            )
            .await?;

        let exec_id = exec.id.clone();
        let client = self.client.clone();

        match self.client.start_exec(&exec.id, None).await? {
            StartExecResults::Attached { output, input } => {
                let input = Box::pin(input);
                let (exit_tx, exit_rx) = tokio::sync::oneshot::channel();

                tokio::spawn(async move {
                    let mut output = output;
                    while let Some(Ok(msg)) = output.next().await {
                        match msg {
                            LogOutput::StdOut { message } => {
                                let text = String::from_utf8_lossy(&message).to_string();
                                on_output(ExecOutput::Stdout(text));
                            }
                            LogOutput::StdErr { message } => {
                                let text = String::from_utf8_lossy(&message).to_string();
                                on_output(ExecOutput::Stderr(text));
                            }
                            _ => {}
                        }
                    }
                    // Output stream ended — fetch exit code
                    let code = match client.inspect_exec(&exec_id).await {
                        Ok(insp) => insp.exit_code.unwrap_or(0),
                        Err(_) => -1,
                    };
                    let _ = exit_tx.send(code);
                });

                Ok(ExecPipe {
                    input,
                    exit: exit_rx,
                })
            }
            StartExecResults::Detached => {
                Err(anyhow::anyhow!("Exec started detached — expected attached"))
            }
        }
    }

    /// Run a command inside the container and return when it finishes.
    /// Streams stdout/stderr to the callback. Non-interactive.
    pub async fn exec_and_wait(
        &self,
        cmd: Vec<String>,
        on_output: impl Fn(ExecOutput) + Send,
    ) -> anyhow::Result<i64> {
        let exec = self
            .client
            .create_exec(
                &self.id,
                CreateExecOptions {
                    cmd: Some(cmd),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    attach_stdin: Some(false),
                    tty: Some(false),
                    ..Default::default()
                },
            )
            .await?;

        match self.client.start_exec(&exec.id, None).await? {
            StartExecResults::Attached {
                mut output,
                input: _,
            } => {
                while let Some(Ok(msg)) = output.next().await {
                    match msg {
                        LogOutput::StdOut { message } => {
                            let text = String::from_utf8_lossy(&message).to_string();
                            on_output(ExecOutput::Stdout(text));
                        }
                        LogOutput::StdErr { message } => {
                            let text = String::from_utf8_lossy(&message).to_string();
                            on_output(ExecOutput::Stderr(text));
                        }
                        _ => {}
                    }
                }

                let inspect = self.client.inspect_exec(&exec.id).await?;
                let exit_code = inspect.exit_code.unwrap_or(0);
                Ok(exit_code)
            }
            StartExecResults::Detached => {
                Err(anyhow::anyhow!("Exec started detached — expected attached"))
            }
        }
    }
}

/// A log line from the container — either stdout or stderr.
#[derive(Debug, Clone)]
pub enum LogLine {
    Stdout(String),
    Stderr(String),
}

/// Output from an exec'd process inside the container.
#[derive(Debug, Clone)]
pub enum ExecOutput {
    Stdout(String),
    Stderr(String),
}

/// A writable pipe to the stdin of an exec'd process inside the container.
/// The `exit` receiver resolves with the process exit code when it terminates.
pub struct ExecPipe {
    input: std::pin::Pin<Box<dyn tokio::io::AsyncWrite + Send>>,
    pub exit: tokio::sync::oneshot::Receiver<i64>,
}

impl ExecPipe {
    pub async fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        use tokio::io::AsyncWriteExt;
        self.input.write_all(data).await?;
        self.input.flush().await?;
        Ok(())
    }

    pub async fn write_line(&mut self, line: &str) -> anyhow::Result<()> {
        self.write(line.as_bytes()).await?;
        self.write(b"\n").await?;
        Ok(())
    }
}

pub struct SandboxManager {
    client: Docker,
    runtime: ContainerRuntime,
    /// The socket/endpoint path the client is connected to (for diagnostics).
    socket: String,
}

impl SandboxManager {
    /// Create a `SandboxManager` by auto-detecting an available container runtime.
    ///
    /// Probes for Docker and Podman sockets in order of preference. See
    /// [`ContainerRuntime::detect`] for detection order and environment overrides.
    pub fn new() -> anyhow::Result<Self> {
        let (runtime, socket) = ContainerRuntime::detect()?;
        let client = Self::connect(&socket)?;
        Ok(Self {
            client,
            runtime,
            socket,
        })
    }

    /// Create a `SandboxManager` connected to an explicit runtime + endpoint.
    ///
    /// Use this when the caller has already resolved the runtime (e.g. from
    /// config) and does not want auto-detection.
    pub fn with_runtime(
        runtime: ContainerRuntime,
        socket: impl Into<String>,
    ) -> anyhow::Result<Self> {
        let socket = socket.into();
        let client = Self::connect(&socket)?;
        Ok(Self {
            client,
            runtime,
            socket,
        })
    }

    /// Which container runtime this manager is connected to.
    pub fn runtime(&self) -> ContainerRuntime {
        self.runtime
    }

    /// The socket/endpoint path the client is connected to.
    pub fn socket(&self) -> &str {
        &self.socket
    }

    /// Construct a bollard client for the given endpoint.
    ///
    /// On Unix, connects to a Unix socket path. On non-Unix (Windows/macOS),
    /// delegates to bollard's platform-aware `connect_with_local_defaults()`
    /// for standard Docker Desktop / named-pipe connections.
    #[cfg(unix)]
    fn connect(socket: &str) -> anyhow::Result<Docker> {
        // DOCKER_HOST-style URLs or "default" — let bollard resolve the endpoint.
        if socket == "default"
            || socket.starts_with("unix://")
            || socket.starts_with("tcp://")
            || socket.starts_with("http://")
            || socket.starts_with("https://")
            || socket.starts_with("npipe://")
        {
            Ok(Docker::connect_with_local_defaults()?)
        } else {
            // Bare socket path (e.g. /var/run/docker.sock).
            Ok(Docker::connect_with_unix(
                socket,
                120,
                bollard::API_DEFAULT_VERSION,
            )?)
        }
    }

    #[cfg(not(unix))]
    fn connect(socket: &str) -> anyhow::Result<Docker> {
        // On Windows, bollard's defaults handle Docker Desktop's named pipe.
        // Custom npipe:// or tcp:// endpoints are handled via DOCKER_HOST.
        let _ = socket; // acknowledged; bollard handles the platform default.
        Ok(Docker::connect_with_local_defaults()?)
    }

    /// Check if a Docker image exists locally.
    pub async fn image_exists(&self, image: &str) -> bool {
        self.client.inspect_image(image).await.is_ok()
    }

    /// Pull an image from registry. Prints progress via callback.
    pub async fn pull_image<F>(&self, image: &str, on_progress: F) -> anyhow::Result<()>
    where
        F: Fn(&str),
    {
        use bollard::image::CreateImageOptions;

        let mut stream = self.client.create_image(
            Some(CreateImageOptions::<String> {
                from_image: image.to_string(),
                ..Default::default()
            }),
            None,
            None,
        );

        while let Some(msg) = stream.next().await {
            match msg {
                Ok(info) => {
                    if let Some(status) = &info.status {
                        on_progress(status);
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Pull failed: {:?}", e));
                }
            }
        }
        Ok(())
    }

    pub async fn create(&self, config: SandboxConfig) -> anyhow::Result<SandboxHandle> {
        let id = format!("sandbox-{}", Uuid::new_v4());
        let host_config = HostConfig {
            mounts: Some(
                config
                    .mounts
                    .iter()
                    .map(|m| bollard::models::Mount {
                        target: Some(m.target.clone()),
                        source: Some(m.source.clone()),
                        read_only: Some(m.read_only),
                        typ: Some(MountTypeEnum::BIND),
                        ..Default::default()
                    })
                    .collect(),
            ),
            memory: config.resources.memory_mb.map(|m| m * 1024 * 1024),
            cpu_count: config.resources.cpu_count,
            network_mode: Some(config.network_mode.clone()),
            ..Default::default()
        };

        let container_config = Config {
            image: Some(config.image.clone()),
            cmd: Some(config.cmd.clone()),
            env: Some(
                config
                    .env
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect(),
            ),
            working_dir: config.working_dir.clone(),
            host_config: Some(host_config),
            ..Default::default()
        };

        self.client
            .create_container(
                Some(CreateContainerOptions {
                    name: id.clone(),
                    ..Default::default()
                }),
                container_config,
            )
            .await?;

        Ok(SandboxHandle::new(id, self.client.clone()))
    }
}
