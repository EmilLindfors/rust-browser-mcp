use std::fmt;

#[derive(Debug)]
pub enum WebDriverError {
    Client(fantoccini::error::CmdError),
    Session(String),
    ElementNotFound { selector: String },
    Timeout { selector: String },
    Generic(anyhow::Error),
}

impl fmt::Display for WebDriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Client(e) => write!(f, "WebDriver client error: {e}"),
            Self::Session(msg) => write!(f, "Session error: {msg}"),
            Self::ElementNotFound { selector } => write!(f, "Element not found: {selector}"),
            Self::Timeout { selector } => write!(f, "Timeout waiting for element: {selector}"),
            Self::Generic(e) => write!(f, "Generic error: {e}"),
        }
    }
}

impl std::error::Error for WebDriverError {}

impl From<fantoccini::error::CmdError> for WebDriverError {
    fn from(err: fantoccini::error::CmdError) -> Self {
        Self::Client(err)
    }
}

impl From<anyhow::Error> for WebDriverError {
    fn from(err: anyhow::Error) -> Self {
        Self::Generic(err)
    }
}

impl From<fantoccini::error::NewSessionError> for WebDriverError {
    fn from(err: fantoccini::error::NewSessionError) -> Self {
        Self::Generic(anyhow::anyhow!("WebDriver session creation error: {}", err))
    }
}

pub type Result<T> = std::result::Result<T, WebDriverError>;
