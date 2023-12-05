use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum PZEMError {
    #[error("Misc error: {0}")]
    Misc(String),
    #[error("Transient Error reported")]
    TransientError,
}