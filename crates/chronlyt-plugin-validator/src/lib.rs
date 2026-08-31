//! Byte-only plugin validation: no staging, network, or guest execution.

mod component;
mod package;

pub use component::validate_component_contract;
pub use package::{PackageSummary, ValidatedPackage, inspect_package, validate_package};

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error(transparent)]
    Contract(#[from] chronlyt_plugin_contracts::ContractError),
    #[error("invalid plugin archive")]
    Archive,
    #[error("plugin integrity mismatch")]
    Integrity,
    #[error("incompatible plugin component")]
    Component,
}

pub type ValidationResult<T> = Result<T, ValidationError>;

impl From<zip::result::ZipError> for ValidationError {
    fn from(_: zip::result::ZipError) -> Self {
        Self::Archive
    }
}

impl From<std::io::Error> for ValidationError {
    fn from(_: std::io::Error) -> Self {
        Self::Archive
    }
}
