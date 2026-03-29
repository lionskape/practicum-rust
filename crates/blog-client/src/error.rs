use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlogClientError {
    #[error("HTTP error: {0}")]
    Http(String),

    #[error("gRPC error: {0}")]
    Grpc(String),

    #[error("transport error: {0}")]
    Transport(String),

    #[error("API error: {status} - {message}")]
    Api { status: u16, message: String },

    #[error("deserialization error: {0}")]
    Deserialization(String),
}

impl From<reqwest::Error> for BlogClientError {
    fn from(err: reqwest::Error) -> Self {
        BlogClientError::Http(err.to_string())
    }
}

impl From<tonic::Status> for BlogClientError {
    fn from(err: tonic::Status) -> Self {
        BlogClientError::Grpc(format!("{}: {}", err.code(), err.message()))
    }
}

impl From<tonic::transport::Error> for BlogClientError {
    fn from(err: tonic::transport::Error) -> Self {
        BlogClientError::Transport(err.to_string())
    }
}
