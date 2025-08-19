use thiserror::Error;

#[derive(Error, Debug)]
pub enum PromptError {
    #[error("Unknown module: {0}")]
    UnknownModule(String),

    #[error("Style error for module '{module}': {error}")]
    StyleError { module: String, error: String },

    #[error("Invalid format '{format}' for module '{module}'. Valid formats: {valid_formats}")]
    InvalidFormat {
        module: String,
        format: String,
        valid_formats: String,
    },

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("UTF-8 conversion error")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

pub type Result<T> = std::result::Result<T, PromptError>;
