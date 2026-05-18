use std::io;
use std::process::{ExitStatus, Output};

use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("startup failed: {0}")]
    Startup(String),
    #[error("logger initialization failed: {0}")]
    LoggerInit(#[from] tracing_subscriber::filter::ParseError),
    #[error("logger initialization failed: {0}")]
    LoggerSet(#[from] tracing::subscriber::SetGlobalDefaultError),
    #[error("failed to spawn process `{program}`: {source}")]
    ProcessSpawn { program: String, source: io::Error },
    #[error(
        "process `{program}` exited with code {code:?}\nstdout:\n{stdout}\nstderr:\n{stderr}"
    )]
    ProcessFailed {
        program: String,
        code: Option<i32>,
        stdout: String,
        stderr: String,
    },
}

impl AppError {
    pub fn startup(message: impl Into<String>) -> Self {
        Self::Startup(message.into())
    }

    pub fn process_spawn(program: impl Into<String>, source: io::Error) -> Self {
        Self::ProcessSpawn {
            program: program.into(),
            source,
        }
    }

    pub fn process_failed(program: impl Into<String>, output: Output) -> Self {
        Self::ProcessFailed {
            program: program.into(),
            code: exit_code(output.status),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

fn exit_code(status: ExitStatus) -> Option<i32> {
    status.code()
}
