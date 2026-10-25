use std::fmt;

#[derive(Debug)]
pub struct VerifyError(pub String);

impl VerifyError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for VerifyError {}

pub type VerifyResult<T> = Result<T, VerifyError>;

pub fn err(message: impl Into<String>) -> VerifyError {
    VerifyError::new(message)
}

impl From<std::io::Error> for VerifyError {
    fn from(value: std::io::Error) -> Self {
        Self(value.to_string())
    }
}

impl From<serde_json::Error> for VerifyError {
    fn from(value: serde_json::Error) -> Self {
        Self(format!("JSON error: {value}"))
    }
}

impl From<sqlx::Error> for VerifyError {
    fn from(value: sqlx::Error) -> Self {
        Self(format!("database error: {value}"))
    }
}
