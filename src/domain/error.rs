use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AppError {
    NotFound(String),
    AuthFailed(String),
    Io(String),
    Protocol(String),
    InvalidInput(String),
    AlreadyExists(String),
    Internal(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NotFound(msg) => write!(f, "Not found: {msg}"),
            AppError::AuthFailed(msg) => write!(f, "Authentication failed: {msg}"),
            AppError::Io(msg) => write!(f, "IO error: {msg}"),
            AppError::Protocol(msg) => write!(f, "Protocol error: {msg}"),
            AppError::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
            AppError::AlreadyExists(msg) => write!(f, "Already exists: {msg}"),
            AppError::Internal(msg) => write!(f, "Internal error: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_error_display() {
        assert_eq!(
            AppError::NotFound("file".into()).to_string(),
            "Not found: file"
        );
        assert_eq!(
            AppError::AuthFailed("bad creds".into()).to_string(),
            "Authentication failed: bad creds"
        );
        assert_eq!(
            AppError::Io("disk full".into()).to_string(),
            "IO error: disk full"
        );
        assert_eq!(
            AppError::Protocol("timeout".into()).to_string(),
            "Protocol error: timeout"
        );
        assert_eq!(
            AppError::InvalidInput("bad".into()).to_string(),
            "Invalid input: bad"
        );
        assert_eq!(
            AppError::AlreadyExists("file".into()).to_string(),
            "Already exists: file"
        );
        assert_eq!(
            AppError::Internal("oops".into()).to_string(),
            "Internal error: oops"
        );
    }

    #[test]
    fn test_app_error_serialize() {
        let err = AppError::NotFound("missing".into());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("notFound"));
        assert!(json.contains("missing"));
    }

    #[test]
    fn test_app_error_from_anyhow() {
        let app_err: AppError = anyhow::anyhow!("something went wrong").into();
        match app_err {
            AppError::Internal(msg) => assert_eq!(msg, "something went wrong"),
            _ => panic!("expected Internal variant"),
        }
    }

    #[test]
    fn test_app_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let app_err: AppError = io_err.into();
        match app_err {
            AppError::Io(msg) => assert!(msg.contains("file not found")),
            _ => panic!("expected Io variant"),
        }
    }
}
