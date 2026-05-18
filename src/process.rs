#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use crate::error::{AppError, Result};

#[derive(Debug, Clone)]
pub struct CommandRequest {
    pub program: String,
    pub args: Vec<String>,
    pub current_dir: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
}

impl CommandRequest {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            current_dir: None,
            env: BTreeMap::new(),
        }
    }

    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_current_dir(mut self, current_dir: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(current_dir.into());
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Default, Clone)]
pub struct ProcessRunner;

impl ProcessRunner {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self, request: &CommandRequest) -> Result<CommandOutput> {
        tracing::debug!(
            program = %request.program,
            args = ?request.args,
            current_dir = ?request.current_dir,
            "running external command"
        );

        let mut command = Command::new(&request.program);
        command.args(&request.args);

        if let Some(current_dir) = &request.current_dir {
            command.current_dir(current_dir);
        }

        if !request.env.is_empty() {
            command.envs(&request.env);
        }

        let output = command
            .output()
            .map_err(|source| AppError::process_spawn(request.program.clone(), source))?;

        if !output.status.success() {
            return Err(AppError::process_failed(request.program.clone(), output));
        }

        Ok(CommandOutput {
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct DockerCommandRunner {
    runner: ProcessRunner,
    docker_program: String,
    default_args: Vec<String>,
}

impl DockerCommandRunner {
    pub fn new(docker_program: impl Into<String>, default_args: Vec<String>) -> Self {
        Self {
            runner: ProcessRunner::new(),
            docker_program: docker_program.into(),
            default_args,
        }
    }

    pub fn run<I, S>(&self, args: I) -> Result<CommandOutput>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut full_args = self.default_args.clone();
        full_args.extend(args.into_iter().map(Into::into));

        let request = CommandRequest::new(self.docker_program.clone()).with_args(full_args);
        self.runner.run(&request)
    }
}
