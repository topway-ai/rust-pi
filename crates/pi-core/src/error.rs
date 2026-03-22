use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("tool execution failed: {0}")]
    ToolFailed(String),

    #[error("provider error: {0}")]
    Provider(String),

    #[error("session error: {0}")]
    Session(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
