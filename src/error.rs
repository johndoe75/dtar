use thiserror;

/// Represents the set of errors that can occur in the `dtar` application.
///
/// This enum is derived from `Debug` and `thiserror::Error` to provide detailed error
/// messages for use with the `?` operator and other error-handling mechanisms.
///
/// # Variants
///
/// - `Io`:
///   Indicates an I/O error occurred while processing the operation.
///   This variant wraps the underlying `std::io::Error`.
///
/// - `NoDirectories`:
///   Indicates that no directories were provided as input. This error
///   is typically raised when the application expects directories to be specified,
///   but none are found.
///
/// # Usage
///
/// The `DtarError` enum allows for comprehensive error handling by integrating
/// with Rust's standard error-handling traits. It provides meaningful error messages
/// to assist both developers and end users in diagnosing issues related to the `dtar` application's operations.
#[derive(Debug, thiserror::Error)]
pub enum DtarError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("No directories provided")]
    NoDirectories,
}
