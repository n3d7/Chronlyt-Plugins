use crate::{
    limits::*,
    urls::{validate_artifact_url, validate_repository_url},
};
use std::collections::BTreeSet;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

use crate::{
    Capability, CapabilitySet, ContractError, ContractResult, PluginId, manifest::validate_sha256,
    manifest::validate_text,
};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CatalogArtifactV1 {
    pub url: String,
    pub sha256: String,
    #[cfg_attr(feature = "schema", schemars(extend("minimum" = 1, "maximum" = MAX_PACKAGE_BYTES)))]
    pub size: u64,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PluginCatalogEntryV1 {
    pub id: PluginId,
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_NAME_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_CATALOG_SUMMARY_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
    pub summary: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_CATALOG_DESCRIPTION_BYTES)))]
    pub description: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_AUTHOR_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
    pub author: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_AUTHOR_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
    pub publisher: String,
    pub repository_url: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_CATALOG_LICENSE_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
    pub license: String,
    #[cfg_attr(feature = "schema", schemars(extend("maxItems" = MAX_CATALOG_CATEGORIES)))]
    pub categories: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("maxItems" = MAX_CATALOG_TAGS)))]
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: String,
    pub chronlyt_compatibility: String,
    #[cfg_attr(feature = "schema", schemars(extend("const" = 1)))]
    pub api_version: u16,
    pub artifact: CatalogArtifactV1,
    #[cfg_attr(feature = "schema", schemars(extend("maxItems" = MAX_PERMISSIONS, "uniqueItems" = true)))]
    pub permissions: Vec<Capability>,
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_CATALOG_CHANGELOG_BYTES)))]
    pub changelog: String,
    pub downloads: Option<u64>,
    pub stars: Option<u64>,
    pub verified: bool,
    pub official: bool,
}

impl PluginCatalogEntryV1 {
    pub fn verify_manifest(&self, manifest: &crate::PluginManifestV1) -> ContractResult<()> {
        let catalog_permissions = CapabilitySet::from_slice(&self.permissions)?;
        let manifest_permissions = CapabilitySet::from_slice(&manifest.permissions)?;
        if self.id != manifest.id
            || Version::parse(&self.version)? != Version::parse(&manifest.version)?
            || self.api_version != manifest.api_version
            || catalog_permissions != manifest_permissions
        {
            return Err(ContractError::InvalidData("catalog manifest mismatch"));
        }
        Ok(())
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_CATALOG_BYTES)))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PluginCatalogV1 {
    #[cfg_attr(feature = "schema", schemars(extend("const" = 1)))]
    pub schema_version: u16,
    pub generated_at: String,
    #[cfg_attr(feature = "schema", schemars(extend("maxItems" = MAX_CATALOG_ENTRIES)))]
    pub plugins: Vec<PluginCatalogEntryV1>,
}

impl PluginCatalogV1 {
    pub fn parse(bytes: &[u8]) -> ContractResult<Self> {
        if bytes.len() > MAX_CATALOG_BYTES {
            return Err(ContractError::ResourceLimit("catalog bytes"));
        }
        let catalog: Self = serde_json::from_slice(bytes)?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn validate(&self) -> ContractResult<()> {
        if self.schema_version != 1 {
            return Err(ContractError::UnsupportedVersion);
        }
        parse_catalog_timestamp(&self.generated_at)?;
        if self.plugins.len() > MAX_CATALOG_ENTRIES {
            return Err(ContractError::ResourceLimit("catalog entries"));
        }
        let mut ids = BTreeSet::new();
        for entry in &self.plugins {
            PluginId::parse(entry.id.as_str())?;
            if !ids.insert(entry.id.as_str()) {
                return Err(ContractError::InvalidData("duplicate catalog plugin id"));
            }
            validate_text(&entry.name, 1, MAX_NAME_BYTES, "catalog name")?;
            validate_text(
                &entry.summary,
                1,
                MAX_CATALOG_SUMMARY_BYTES,
                "catalog summary",
            )?;
            validate_text(
                &entry.description,
                0,
                MAX_CATALOG_DESCRIPTION_BYTES,
                "catalog description",
            )?;
            validate_text(&entry.author, 1, MAX_AUTHOR_BYTES, "catalog author")?;
            validate_text(&entry.publisher, 1, MAX_AUTHOR_BYTES, "catalog publisher")?;
            validate_text(
                &entry.license,
                1,
                MAX_CATALOG_LICENSE_BYTES,
                "catalog license",
            )?;
            validate_text(
                &entry.changelog,
                0,
                MAX_CATALOG_CHANGELOG_BYTES,
                "catalog changelog",
            )?;
            validate_catalog_labels(
                &entry.categories,
                MAX_CATALOG_CATEGORIES,
                MAX_CATALOG_LABEL_BYTES,
                "catalog categories",
            )?;
            validate_catalog_labels(
                &entry.tags,
                MAX_CATALOG_TAGS,
                MAX_CATALOG_LABEL_BYTES,
                "catalog tags",
            )?;
            parse_catalog_timestamp(&entry.created_at)?;
            parse_catalog_timestamp(&entry.updated_at)?;
            Version::parse(&entry.version)?;
            VersionReq::parse(&entry.chronlyt_compatibility)?;
            if entry.api_version != 1 {
                return Err(ContractError::UnsupportedVersion);
            }
            validate_repository_url(&entry.repository_url)?;
            validate_artifact_url(&entry.artifact.url)?;
            validate_sha256(&entry.artifact.sha256)?;
            if entry.artifact.size == 0 || entry.artifact.size > MAX_PACKAGE_BYTES as u64 {
                return Err(ContractError::ResourceLimit("artifact bytes"));
            }
            CapabilitySet::from_slice(&entry.permissions)?;
        }
        Ok(())
    }
}

pub fn validate_catalog_labels(
    values: &[String],
    max_items: usize,
    max_item_bytes: usize,
    label: &'static str,
) -> ContractResult<()> {
    if values.len() > max_items {
        return Err(ContractError::ResourceLimit(label));
    }
    let mut unique = BTreeSet::new();
    for value in values {
        validate_text(value, 1, max_item_bytes, label)?;
        if !unique.insert(value.as_str()) {
            return Err(ContractError::InvalidData("duplicate catalog label"));
        }
    }
    Ok(())
}

pub fn parse_catalog_timestamp(
    value: &str,
) -> ContractResult<chrono::DateTime<chrono::FixedOffset>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|_| ContractError::InvalidData("catalog timestamp"))
}
