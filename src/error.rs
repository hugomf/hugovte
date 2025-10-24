// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TerminalError {
    #[error("Failed to create drawing cache: {0}")]
    DrawingCacheCreation(String),
    
    #[error("PTY error: {0}")]
    PtyError(#[from] portable_pty::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Grid lock error: {0}")]
    GridLockError(String),
    
    #[error("Channel send error")]
    ChannelSendError,
    
    #[error("Font error: {0}")]
    FontError(String),
}

pub type TerminalResult<T> = Result<T, TerminalError>;