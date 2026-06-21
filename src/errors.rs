use thiserror::Error;


#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Database Error: {0}")]
    Database(String),

    #[error("Upload Error: {0}")]
    Upload(String),

    #[error("Invalid Error: {0}")]
    Invalid(String),

    #[error("Encode Error: {0}")]
    Encode(String),

    #[error("Decode Error: {0}")]
    Decode(String),
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Provider {0} not found")]
    NotFound(String),

    #[error("Provider {0} no api keys")]
    NoApiKeys(String),

    #[error("Provider {0} request failed {1}: {2}")]
    Http(String, u16, String),
}


#[derive(Debug, Error)]
pub enum ProducerError {
    #[error("Producer {0} generate error: {1}")]
    Generate(String, String),
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Unauthorized Error: {0}")]
    Unauthorized(String),

    #[error("QuotaExceeded Error: {1}s {0}")]
    QuotaExceeded(String, u64),

    #[error("RateLimited Error: {1}s {0}")]
    RateLimited(String, u64),

    #[error("InvalidResponse Error: {0}")]
    InvalidResponse(String),

    #[error("InvalidApiKey Error: {0}")]
    InvalidApiKey(String),

    #[error("Internal Error: {0}")]
    Internal(String),

    #[error("Network Error: {0}")]
    Network(String),
}
