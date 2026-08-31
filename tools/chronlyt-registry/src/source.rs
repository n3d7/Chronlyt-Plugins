use crate::{RegistryError, RegistryResult};
use chronlyt_plugin_contracts::{
    PluginId, limits::*, parse_catalog_timestamp, validate_artifact_url, validate_catalog_labels,
    validate_repository_url, validate_text,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

// Registry intake policy, not additional Chronlyt wire limits.
pub const MAX_SOURCE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_RECORD_BYTES: usize = 64 * 1024;
pub const MAX_METADATA_BYTES: usize = 4 * 1024;

pub(crate) fn record_identity(name: &str) -> RegistryResult<Option<PluginId>> {
    if name == ".gitkeep" {
        return Ok(None);
    }
    let id = name
        .strip_suffix(".json")
        .ok_or(RegistryError::Invalid("record filename"))?;
    Ok(Some(PluginId::parse(id)?))
}

#[derive(schemars::JsonSchema)]
#[schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_METADATA_BYTES))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryMetadataV1 {
    #[schemars(extend("const" = 1))]
    pub schema_version: u16,
    pub generated_at: String,
}

#[derive(schemars::JsonSchema, Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryReleaseV1 {
    pub version: String,
    pub artifact_url: String,
}

#[derive(schemars::JsonSchema)]
#[schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_RECORD_BYTES))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryRecordV1 {
    #[schemars(extend("const" = 1))]
    pub schema_version: u16,
    pub id: PluginId,
    pub release: RegistryReleaseV1,
    pub repository_url: String,
    #[schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_CATALOG_SUMMARY_BYTES, "x-chronlyt-min-utf8-bytes" = 1))]
    pub summary: String,
    #[schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_AUTHOR_BYTES, "x-chronlyt-min-utf8-bytes" = 1))]
    pub publisher: String,
    #[schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_CATALOG_LICENSE_BYTES, "x-chronlyt-min-utf8-bytes" = 1))]
    pub license: String,
    #[schemars(extend("maxItems" = MAX_CATALOG_CATEGORIES))]
    pub categories: Vec<String>,
    #[schemars(extend("maxItems" = MAX_CATALOG_TAGS))]
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    #[schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_CATALOG_CHANGELOG_BYTES))]
    pub changelog: String,
}

#[derive(Debug, Clone)]
pub struct RegistrySourceV1 {
    pub metadata: RegistryMetadataV1,
    pub records: Vec<RegistryRecordV1>,
}

impl RegistrySourceV1 {
    /// Shared data-only entrypoint for bounded local/Git/API readers.
    /// Record names are single filenames within registry/plugins, not paths.
    pub fn from_blobs(metadata: &[u8], blobs: &[(String, Vec<u8>)]) -> RegistryResult<Self> {
        if metadata.len() > MAX_METADATA_BYTES || blobs.len() > MAX_CATALOG_ENTRIES + 1 {
            return Err(RegistryError::Limit("source records"));
        }
        let mut total = metadata.len();
        let metadata: RegistryMetadataV1 = serde_json::from_slice(metadata)?;
        let mut names = BTreeSet::new();
        let mut records = Vec::new();
        for (name, bytes) in blobs {
            if !names.insert(name.as_str()) {
                return Err(RegistryError::Invalid("duplicate source name"));
            }
            if bytes.len() > MAX_RECORD_BYTES {
                return Err(RegistryError::Limit("record bytes"));
            }
            total = total
                .checked_add(bytes.len())
                .ok_or(RegistryError::Limit("source bytes"))?;
            if total > MAX_SOURCE_BYTES {
                return Err(RegistryError::Limit("source bytes"));
            }
            let Some(id) = record_identity(name)? else {
                if bytes.is_empty() {
                    continue;
                }
                return Err(RegistryError::Invalid("nonempty sentinel"));
            };
            let record: RegistryRecordV1 = serde_json::from_slice(bytes)?;
            if id != record.id {
                return Err(RegistryError::Invalid("filename identity mismatch"));
            }
            records.push(record);
        }
        let source = Self { metadata, records };
        source.validate()?;
        Ok(source)
    }

    pub(crate) fn validate(&self) -> RegistryResult<()> {
        if self.metadata.schema_version != 1 {
            return Err(RegistryError::Invalid("source schema version"));
        }
        let generated = parse_catalog_timestamp(&self.metadata.generated_at)?;
        if self.records.len() > MAX_CATALOG_ENTRIES {
            return Err(RegistryError::Limit("records"));
        }
        let mut ids = BTreeSet::new();
        for record in &self.records {
            if record.schema_version != 1 {
                return Err(RegistryError::Invalid("source schema version"));
            }
            PluginId::parse(record.id.as_str())?;
            if !ids.insert(record.id.as_str()) {
                return Err(RegistryError::Invalid("duplicate record identity"));
            }
            semver::Version::parse(&record.release.version)?;
            validate_artifact_url(&record.release.artifact_url)?;
            validate_repository_url(&record.repository_url)?;
            validate_text(
                &record.summary,
                1,
                MAX_CATALOG_SUMMARY_BYTES,
                "catalog summary",
            )?;
            validate_text(&record.publisher, 1, MAX_AUTHOR_BYTES, "catalog publisher")?;
            validate_text(
                &record.license,
                1,
                MAX_CATALOG_LICENSE_BYTES,
                "catalog license",
            )?;
            validate_text(
                &record.changelog,
                0,
                MAX_CATALOG_CHANGELOG_BYTES,
                "catalog changelog",
            )?;
            validate_catalog_labels(
                &record.categories,
                MAX_CATALOG_CATEGORIES,
                MAX_CATALOG_LABEL_BYTES,
                "catalog categories",
            )?;
            validate_catalog_labels(
                &record.tags,
                MAX_CATALOG_TAGS,
                MAX_CATALOG_LABEL_BYTES,
                "catalog tags",
            )?;
            let created = parse_catalog_timestamp(&record.created_at)?;
            let updated = parse_catalog_timestamp(&record.updated_at)?;
            if created > updated || updated > generated {
                return Err(RegistryError::Invalid("timestamp ordering"));
            }
        }
        Ok(())
    }
}
