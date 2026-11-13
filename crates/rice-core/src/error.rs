//! Error payload of rejected commands (`BackendError` in contract.ts).

use serde::Serialize;
use std::fmt;

/// Machine-readable error categories the UI knows how to present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    NotSupported,
    PermissionRequired,
    NotFound,
    InvalidArgs,
    AuthFailed,
    Network,
    Timeout,
    AccessDenied,
    Internal,
}

/// Error payload of a rejected command, serialised as `{ "code": …, "message": … }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BackendError {
    pub code: ErrorCode,
    pub message: String,
}

/// Result type of every command.
pub type BackendResult<T> = Result<T, BackendError>;

impl BackendError {
    /// Creates an error with the given code.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }

    /// The feature does not exist on this machine or is not built yet.
    pub fn not_supported(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotSupported, message)
    }

    /// Input failed validation.
    pub fn invalid_args(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidArgs, message)
    }

    /// The addressed item does not exist.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    /// Windows refused the operation.
    pub fn access_denied(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::AccessDenied, message)
    }

    /// Anything unexpected; the message is for logs, not for the UI.
    pub fn internal(error: impl fmt::Display) -> Self {
        Self::new(ErrorCode::Internal, error.to_string())
    }
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for BackendError {}

impl From<std::io::Error> for BackendError {
    fn from(error: std::io::Error) -> Self {
        use std::io::ErrorKind;
        let code = match error.kind() {
            ErrorKind::NotFound => ErrorCode::NotFound,
            ErrorKind::PermissionDenied => ErrorCode::AccessDenied,
            ErrorKind::TimedOut => ErrorCode::Timeout,
            _ => ErrorCode::Internal,
        };
        Self::new(code, error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serialises_like_the_contract() {
        let error = BackendError::invalid_args("bad id");
        assert_eq!(serde_json::to_value(&error).unwrap(), json!({ "code": "invalid_args", "message": "bad id" }));
        let codes = [
            (ErrorCode::NotSupported, "not_supported"),
            (ErrorCode::PermissionRequired, "permission_required"),
            (ErrorCode::NotFound, "not_found"),
            (ErrorCode::AuthFailed, "auth_failed"),
            (ErrorCode::Network, "network"),
            (ErrorCode::Timeout, "timeout"),
            (ErrorCode::AccessDenied, "access_denied"),
            (ErrorCode::Internal, "internal"),
        ];
        for (code, wire) in codes {
            assert_eq!(serde_json::to_value(code).unwrap(), json!(wire));
        }
    }

    #[test]
    fn maps_io_errors() {
        let missing = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        assert_eq!(BackendError::from(missing).code, ErrorCode::NotFound);
        let denied = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "no");
        assert_eq!(BackendError::from(denied).code, ErrorCode::AccessDenied);
    }
}
