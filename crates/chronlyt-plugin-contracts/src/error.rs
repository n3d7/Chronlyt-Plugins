use thiserror::Error;

/// Bounded protocol errors: never carry untrusted payloads, paths or provider bodies.
#[derive(Debug, Error)]
pub enum ContractError {
    #[error("invalid plugin data: {0}")]
    InvalidData(&'static str),
    #[error("unsupported plugin protocol version")]
    UnsupportedVersion,
    #[error("plugin resource limit exceeded: {0}")]
    ResourceLimit(&'static str),
    #[error("plugin data could not be decoded")]
    Decode,
    #[error("plugin semantic version is invalid")]
    Version,
    #[error("plugin URL is invalid")]
    Url,
}

impl From<serde_json::Error> for ContractError {
    fn from(_: serde_json::Error) -> Self {
        Self::Decode
    }
}
impl From<semver::Error> for ContractError {
    fn from(_: semver::Error) -> Self {
        Self::Version
    }
}
impl From<url::ParseError> for ContractError {
    fn from(_: url::ParseError) -> Self {
        Self::Url
    }
}

pub type ContractResult<T> = Result<T, ContractError>;
