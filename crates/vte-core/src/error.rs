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
