//! Internal registry data validation and deterministic catalog generation.

pub mod commands;
mod error;
mod generate;
pub mod git_data;
pub mod schema_export;
mod source;
pub mod transport;

pub use error::{RegistryError, RegistryResult};
pub use generate::{ArtifactFetcher, FetchBudget, generate_catalog};
pub use git_data::{read_source_directory, read_source_git, read_source_pr};
pub use source::{RegistryMetadataV1, RegistryRecordV1, RegistryReleaseV1, RegistrySourceV1};
