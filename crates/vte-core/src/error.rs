// src/error.rs
use thiserror::Error;

/// Comprehensive error hierarchy for the VTE terminal
/// Covers all failure modes from PTY creation to rendering and user interaction
#[derive(Error, Debug)]
pub enum TerminalError {
    // PTY and Process Management Errors
    #[error("PTY creation failed: {message}")]
    PtyCreationFailed { message: String },

    #[error("PTY connection lost: {message}")]
    PtyDisconnected { message: String },

    #[error("PTY reader error: {source}")]
    PtyReadError {
        #[from]
        source: std::io::Error,
    },

    #[error("Failed to spawn shell process: {program}")]
    ProcessSpawnFailed { program: String },

    // Grid and Rendering Errors
    #[error("Grid lock poisoned: {message}")]
    GridLockError { message: String },

    #[error("Invalid grid coordinates: row={row}, col={col} in {rows}x{cols} grid")]
    InvalidCoordinates { row: usize, col: usize, rows: usize, cols: usize },

    #[error("Buffer operation failed: {message}")]
    BufferOperationFailed { message: String },

    // Rendering and Drawing Errors
    #[error("Failed to create drawing cache: {message}")]
    DrawingCacheCreationFailed { message: String },

    #[error("Font error: {message}")]
    FontError { message: String },

    #[error("Render error: {adapter}, {message}")]
    RenderingFailed { adapter: String, message: String },

    // Input and Interaction Errors
    #[error("Input handling failed: {message}")]
    InputError { message: String },

    #[error("Clipboard operation failed: {operation}")]
    ClipboardError { operation: String },

    #[error("Selection operation failed: {message}")]
    SelectionError { message: String },

    // Configuration and Initialization Errors
    #[error("Invalid configuration: {field} = {value}")]
    ConfigurationError { field: String, value: String },

    #[error("Terminal initialization failed: {reason}")]
    InitializationError { reason: String },

    // Communication and Synchronization Errors
    #[error("Channel send failed: {destination}")]
    ChannelSendError { destination: String },

    #[error("Async runtime error: channel closed")]
    RuntimeError,

    // Parser and Protocol Errors (expanded from AnsiError)
    #[error("ANSI/VT parser error: {message}")]
    ParserError { message: String },

    #[error("Invalid escape sequence: {sequence}")]
    InvalidEscapeSequence { sequence: String },

    #[error("OS command injection attempt detected: {command}")]
    OsCommandInjection { command: String },

    // Resource and Memory Errors
    #[error("Memory limit exceeded: tried to allocate {requested} bytes, limit is {limit}")]
    MemoryLimitExceeded { requested: usize, limit: usize },

    #[error("Resource cleanup failed: {resource}")]
    ResourceCleanupFailed { resource: String },

    // Generic fallback for unexpected errors
    #[error("Unexpected internal error: {message}")]
    InternalError { message: String },
}

pub type TerminalResult<T> = Result<T, TerminalError>;

/// Enhanced error recovery strategies for terminal operations
impl TerminalError {
    /// Check if this error is recoverable through automatic retry
    pub fn is_recoverable(&self) -> bool {
        matches!(self,
            TerminalError::PtyReadError { .. } |
            TerminalError::GridLockError { .. } |
            TerminalError::ChannelSendError { .. } |
            TerminalError::BufferOperationFailed { .. }
        )
    }

    /// Suggest recovery action for this error type
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            TerminalError::PtyDisconnected { .. } |
            TerminalError::PtyReadError { .. } =>
                RecoveryStrategy::ReconnectPty,

            TerminalError::GridLockError { .. } =>
                RecoveryStrategy::RetryWithTimeout,

            TerminalError::ChannelSendError { .. } =>
                RecoveryStrategy::DropAndReconnect,

            TerminalError::MemoryLimitExceeded { .. } =>
                RecoveryStrategy::CleanupAndRetry,

            TerminalError::ProcessSpawnFailed { .. } =>
                RecoveryStrategy::RetryWithDifferentShell,

            TerminalError::FontError { .. } =>
                RecoveryStrategy::FallbackFont,

            _ => RecoveryStrategy::PropagateError,
        }
    }

    /// Maximum number of retry attempts for this error type
    pub fn max_retry_attempts(&self) -> usize {
        match self.recovery_strategy() {
            RecoveryStrategy::ReconnectPty => 3,
            RecoveryStrategy::RetryWithTimeout => 5,
            RecoveryStrategy::CleanupAndRetry => 2,
            RecoveryStrategy::RetryWithDifferentShell => 1,
            RecoveryStrategy::FallbackFont => 3,
            RecoveryStrategy::DropAndReconnect => 2,
            RecoveryStrategy::PropagateError => 0,
        }
    }

    /// Timeout between retry attempts
    pub fn retry_timeout(&self) -> std::time::Duration {
        match self.recovery_strategy() {
            RecoveryStrategy::ReconnectPty => std::time::Duration::from_millis(500),
            RecoveryStrategy::RetryWithTimeout => std::time::Duration::from_millis(100),
            RecoveryStrategy::CleanupAndRetry => std::time::Duration::from_millis(50),
            RecoveryStrategy::RetryWithDifferentShell => std::time::Duration::from_secs(2),
            RecoveryStrategy::FallbackFont => std::time::Duration::from_millis(10),
            RecoveryStrategy::DropAndReconnect => std::time::Duration::from_millis(200),
            RecoveryStrategy::PropagateError => std::time::Duration::from_millis(0),
        }
    }
}

/// Recovery strategies for different error types
#[derive(Debug, Clone, Copy)]
pub enum RecoveryStrategy {
    /// Attempt to reconnect PTY
    ReconnectPty,
    /// Retry operation after timeout
    RetryWithTimeout,
    /// Drop failed connection and establish new one
    DropAndReconnect,
    /// Clean up resources and retry
    CleanupAndRetry,
    /// Try spawning with different shell
    RetryWithDifferentShell,
    /// Switch to fallback font
    FallbackFont,
    /// Continue with error (no recovery possible)
    PropagateError,
}
