use std::io;

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
    #[error("failed to parse JSON output: {0}")]
    Json(#[from] serde_json::Error),
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

    pub fn process_failed_from_parts(
        program: impl Into<String>,
        code: Option<i32>,
        stdout: impl Into<String>,
        stderr: impl Into<String>,
    ) -> Self {
        Self::ProcessFailed {
            program: program.into(),
            code,
            stdout: stdout.into(),
            stderr: stderr.into(),
        }
    }
}
