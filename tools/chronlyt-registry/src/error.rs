#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error(transparent)]
    Contract(#[from] chronlyt_plugin_contracts::ContractError),
    #[error(transparent)]
    Package(#[from] chronlyt_plugin_validator::ValidationError),
    #[error("invalid registry data: {0}")]
    Invalid(&'static str),
    #[error("registry resource limit: {0}")]
    Limit(&'static str),
    #[error("registry transport failed")]
    Transport,
    #[error("registry input/output failed")]
    Io,
    #[error("registry JSON encoding or decoding failed")]
    Json,
}
pub type RegistryResult<T> = Result<T, RegistryError>;

impl From<serde_json::Error> for RegistryError {
    fn from(_: serde_json::Error) -> Self {
        Self::Json
    }
}
impl From<std::io::Error> for RegistryError {
    fn from(_: std::io::Error) -> Self {
        Self::Io
    }
}
impl From<semver::Error> for RegistryError {
    fn from(_: semver::Error) -> Self {
        Self::Invalid("release version")
    }
}
