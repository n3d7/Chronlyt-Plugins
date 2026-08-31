use std::collections::BTreeSet;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

use crate::{
    Capability, CapabilitySet, ContractError, ContractResult, limits::*,
    paths::validate_relative_path,
};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_PLUGIN_ID_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct PluginId(String);

impl PluginId {
    pub fn parse(value: &str) -> ContractResult<Self> {
        let bytes = value.as_bytes();
        let is_alphanumeric = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
        let valid = !bytes.is_empty()
            && bytes.len() <= MAX_PLUGIN_ID_BYTES
            && value != "core"
            && bytes.first().copied().is_some_and(is_alphanumeric)
            && bytes.last().copied().is_some_and(is_alphanumeric)
            && bytes
                .iter()
                .copied()
                .all(|byte| is_alphanumeric(byte) || matches!(byte, b'.' | b'-' | b'_'))
            && !bytes
                .windows(2)
                .any(|pair| !is_alphanumeric(pair[0]) && !is_alphanumeric(pair[1]));
        if valid {
            Ok(Self(value.to_owned()))
        } else {
            Err(ContractError::InvalidData("plugin id"))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PluginCompatibilityV1 {
    pub chronlyt: String,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PluginComponentV1 {
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_PATH_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
    pub path: String,
    pub sha256: String,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PluginPageManifestV1 {
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_IDENTIFIER_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_NAME_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
    pub title: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_PAGE_DESCRIPTION_BYTES)))]
    pub description: String,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PluginAssetManifestV1 {
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_PATH_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
    pub path: String,
    pub sha256: String,
    pub media_type: String,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_MANIFEST_BYTES)))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PluginManifestV1 {
    #[cfg_attr(feature = "schema", schemars(extend("const" = 1)))]
    pub schema_version: u16,
    #[cfg_attr(feature = "schema", schemars(extend("const" = 1)))]
    pub api_version: u16,
    pub id: PluginId,
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_NAME_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
    pub name: String,
    pub version: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_DESCRIPTION_BYTES)))]
    pub description: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_AUTHOR_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
    pub author: String,
    pub compatibility: PluginCompatibilityV1,
    pub component: PluginComponentV1,
    #[cfg_attr(feature = "schema", schemars(extend("maxItems" = MAX_PERMISSIONS, "uniqueItems" = true)))]
    pub permissions: Vec<Capability>,
    #[cfg_attr(feature = "schema", schemars(extend("minItems" = 1, "maxItems" = MAX_PAGES)))]
    pub pages: Vec<PluginPageManifestV1>,
    #[cfg_attr(feature = "schema", schemars(extend("maxItems" = MAX_ASSETS)))]
    pub assets: Vec<PluginAssetManifestV1>,
}

impl PluginManifestV1 {
    pub fn parse(bytes: &[u8]) -> ContractResult<Self> {
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(ContractError::ResourceLimit("manifest bytes"));
        }
        let manifest: Self = serde_json::from_slice(bytes)?;
        manifest.validate()?;
        Ok(manifest)
    }

    fn validate(&self) -> ContractResult<()> {
        if self.schema_version != 1 || self.api_version != 1 {
            return Err(ContractError::UnsupportedVersion);
        }
        PluginId::parse(self.id.as_str())?;
        validate_text(&self.name, 1, MAX_NAME_BYTES, "plugin name")?;
        validate_text(
            &self.description,
            0,
            MAX_DESCRIPTION_BYTES,
            "plugin description",
        )?;
        validate_text(&self.author, 1, MAX_AUTHOR_BYTES, "plugin author")?;
        Version::parse(&self.version)?;
        VersionReq::parse(&self.compatibility.chronlyt)?;
        validate_relative_path(&self.component.path)?;
        if !self.component.path.ends_with(".wasm") {
            return Err(ContractError::InvalidData("component path"));
        }
        validate_sha256(&self.component.sha256)?;
        CapabilitySet::from_slice(&self.permissions)?;
        if self.pages.is_empty() || self.pages.len() > MAX_PAGES {
            return Err(ContractError::ResourceLimit("page contributions"));
        }
        let mut page_ids = BTreeSet::new();
        for page in &self.pages {
            validate_identifier(&page.id, "page id")?;
            if !page_ids.insert(page.id.as_str()) {
                return Err(ContractError::InvalidData("duplicate page id"));
            }
            validate_text(&page.title, 1, MAX_NAME_BYTES, "page title")?;
            validate_text(
                &page.description,
                0,
                MAX_PAGE_DESCRIPTION_BYTES,
                "page description",
            )?;
        }
        if self.assets.len() > MAX_ASSETS {
            return Err(ContractError::ResourceLimit("asset declarations"));
        }
        let mut asset_paths = BTreeSet::new();
        for asset in &self.assets {
            validate_relative_path(&asset.path)?;
            if asset.path == self.component.path || !asset_paths.insert(asset.path.as_str()) {
                return Err(ContractError::InvalidData("duplicate asset path"));
            }
            validate_sha256(&asset.sha256)?;
            if !matches!(asset.media_type.as_str(), "image/png" | "image/webp") {
                return Err(ContractError::InvalidData("asset media type"));
            }
        }
        Ok(())
    }
}

pub(crate) fn validate_identifier(value: &str, label: &'static str) -> ContractResult<()> {
    let valid = !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        });
    if valid {
        Ok(())
    } else {
        Err(ContractError::InvalidData(label))
    }
}

pub fn validate_text(
    value: &str,
    min_bytes: usize,
    max_bytes: usize,
    label: &'static str,
) -> ContractResult<()> {
    if (min_bytes..=max_bytes).contains(&value.len()) {
        Ok(())
    } else {
        Err(ContractError::ResourceLimit(label))
    }
}

pub fn validate_sha256(value: &str) -> ContractResult<()> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(ContractError::InvalidData("sha256"))
    }
}
