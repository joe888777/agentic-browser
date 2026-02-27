use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Browser launch failed: {0}")]
    LaunchError(String),

    #[error("Navigation failed: {0}")]
    NavigationError(String),

    #[error("Element not found: {0}")]
    ElementNotFound(String),

    #[error("Timeout waiting for: {0}")]
    Timeout(String),

    #[error("JavaScript error: {0}")]
    JsError(String),

    #[error("Screenshot failed: {0}")]
    ScreenshotError(String),

    #[error("CDP error: {0}")]
    CdpError(#[from] chromiumoxide::error::CdpError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
