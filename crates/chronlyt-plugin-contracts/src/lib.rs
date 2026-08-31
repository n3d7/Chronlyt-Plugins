//! Canonical Chronlyt v1 wire contracts, extracted without runtime authority.

mod catalog;
mod error;
pub mod host_payloads;
pub mod limits;
mod manifest;
mod paths;
mod permissions;
mod ui;
mod urls;

pub const WIT_SOURCE: &str = include_str!("../wit/chronlyt-plugin.wit");

pub use catalog::{CatalogArtifactV1, PluginCatalogEntryV1, PluginCatalogV1};
pub use catalog::{parse_catalog_timestamp, validate_catalog_labels};
pub use error::{ContractError, ContractResult};
pub use host_payloads::{
    CreatePluginTimelineEntryInput, PluginStorageEntry, TimelineEntry, TimelineListQuery,
    UpdateTimelineEntryInput,
};
pub use manifest::validate_text;
pub use manifest::{
    PluginAssetManifestV1, PluginCompatibilityV1, PluginComponentV1, PluginId, PluginManifestV1,
    PluginPageManifestV1, validate_sha256,
};
pub use paths::validate_relative_path;
pub use permissions::{Capability, CapabilitySet, PermissionDelta};
pub use ui::{
    PluginButtonVariantV1, PluginListItemV1, PluginNodeV1, PluginSelectOptionV1, PluginSpacingV1,
    PluginToneV1, PluginViewV1,
};
pub use urls::{validate_artifact_url, validate_redirect_url, validate_repository_url};
