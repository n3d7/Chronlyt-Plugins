//! Guest-visible JSON only; no database, plugin identity or grant authority.
use crate::{ContractError, ContractResult, limits::*};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginStorageEntry {
    pub key: String,
    pub value_json: String,
    pub updated_at: String,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineEntry {
    pub id: String,
    pub source_namespace: String,
    pub source_ref: Option<String>,
    pub title: String,
    pub occurred_at: String,
    pub duration_seconds: Option<i64>,
    pub note: String,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreatePluginTimelineEntryInput {
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_SOURCE_REF_BYTES)))]
    pub source_ref: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-trimmed-max-codepoints" = MAX_TITLE_CHARACTERS)))]
    pub title: String,
    pub occurred_at: String,
    #[cfg_attr(feature = "schema", schemars(extend("minimum" = 1, "maximum" = MAX_DURATION_SECONDS)))]
    pub duration_seconds: Option<i64>,
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(extend("maxLength" = MAX_NOTE_CHARACTERS)))]
    pub note: String,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct UpdateTimelineEntryInput {
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-trimmed-max-codepoints" = MAX_TITLE_CHARACTERS)))]
    pub title: Option<String>,
    pub occurred_at: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_patch")]
    #[cfg_attr(feature = "schema", schemars(extend("minimum" = 1, "maximum" = MAX_DURATION_SECONDS)))]
    pub duration_seconds: Option<Option<i64>>,
    #[cfg_attr(feature = "schema", schemars(extend("maxLength" = MAX_NOTE_CHARACTERS)))]
    pub note: Option<String>,
}

fn deserialize_nullable_patch<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineListQuery {
    #[cfg_attr(feature = "schema", schemars(extend("minimum" = 1, "maximum" = MAX_TIMELINE_QUERY_RESULTS)))]
    pub limit: Option<usize>,
}

pub fn validate_timeline_query_limit(limit: Option<usize>) -> ContractResult<usize> {
    let limit = limit.unwrap_or(DEFAULT_TIMELINE_QUERY_RESULTS);
    if !(1..=MAX_TIMELINE_QUERY_RESULTS).contains(&limit) {
        return Err(ContractError::ResourceLimit("timeline result count"));
    }
    Ok(limit)
}

pub fn validate_timeline_title(value: &str) -> ContractResult<&str> {
    let title = value.trim();
    if title.is_empty() || title.chars().count() > MAX_TITLE_CHARACTERS {
        return Err(ContractError::InvalidData("timeline title"));
    }
    Ok(title)
}

pub fn validate_timeline_note(value: &str) -> ContractResult<()> {
    if value.chars().count() > MAX_NOTE_CHARACTERS {
        return Err(ContractError::ResourceLimit("timeline note"));
    }
    Ok(())
}

pub fn validate_timeline_duration(value: Option<i64>) -> ContractResult<()> {
    if value.is_some_and(|seconds| !(1..=MAX_DURATION_SECONDS).contains(&seconds)) {
        return Err(ContractError::InvalidData("timeline duration"));
    }
    Ok(())
}

pub fn validate_timeline_source_ref(value: Option<&str>) -> ContractResult<()> {
    if value
        .is_some_and(|reference| reference.len() > MAX_SOURCE_REF_BYTES || reference.contains('\0'))
    {
        Err(ContractError::InvalidData("timeline source_ref"))
    } else {
        Ok(())
    }
}

pub fn normalize_timeline_timestamp(value: &str) -> ContractResult<String> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc).to_rfc3339())
        .map_err(|_| ContractError::InvalidData("timeline timestamp"))
}

pub fn validate_timeline_record_id(value: &str) -> ContractResult<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(ContractError::InvalidData("timeline entry id"));
    }
    Ok(())
}

pub fn validate_storage_key(value: &str) -> ContractResult<()> {
    if !value.is_empty() && value.len() <= MAX_STORAGE_KEY_BYTES && !value.contains('\0') {
        Ok(())
    } else {
        Err(ContractError::InvalidData("storage key"))
    }
}

pub fn validate_storage_value(value: &str) -> ContractResult<()> {
    if value.len() > MAX_STORAGE_VALUE_BYTES {
        return Err(ContractError::ResourceLimit("storage value"));
    }
    serde_json::from_str::<serde_json::Value>(value)?;
    Ok(())
}
