use thiserror;

#[derive(Debug, thiserror::Error)]
pub enum DtarError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("No directories provided")]
    NoDirectories,
}
