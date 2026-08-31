//! Generated documentation, never a replacement for shared semantic validation.
use crate::{RegistryMetadataV1, RegistryRecordV1, RegistryResult};
use chronlyt_plugin_contracts::{limits::WIRE_LIMITS, *};
use schemars::{JsonSchema, SchemaGenerator, generate::SchemaSettings};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn generator() -> SchemaGenerator {
    SchemaSettings::draft2020_12()
        .for_deserialize()
        .into_generator()
}

fn schema<T: JsonSchema>() -> RegistryResult<Value> {
    Ok(serde_json::to_value(
        generator().into_root_schema_for::<T>(),
    )?)
}

pub fn generated_contracts() -> RegistryResult<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();
    files.insert("manifest.schema.json", schema::<PluginManifestV1>()?);
    files.insert("catalog.schema.json", schema::<PluginCatalogV1>()?);
    files.insert("declarative-ui.schema.json", schema::<PluginViewV1>()?);

    let mut payloads = generator();
    let roots = [
        payloads.subschema_for::<PluginStorageEntry>(),
        payloads.subschema_for::<TimelineEntry>(),
        payloads.subschema_for::<CreatePluginTimelineEntryInput>(),
        payloads.subschema_for::<UpdateTimelineEntryInput>(),
        payloads.subschema_for::<TimelineListQuery>(),
    ];
    files.insert("host-payloads.schema.json", json!({
        "$schema":"https://json-schema.org/draft/2020-12/schema",
        "title":"Chronlyt v1 guest JSON payloads",
        "description":"Select the named $defs type for the specific WIT operation. Input/output wire byte budgets and semantic Rust validation also apply.",
        "anyOf":roots, "$defs":payloads.take_definitions(true)
    }));

    let mut records = generator();
    records.subschema_for::<RegistryMetadataV1>();
    files.insert(
        "registry-source.schema.json",
        serde_json::to_value(records.into_root_schema_for::<RegistryRecordV1>())?,
    );
    let capabilities = schema::<Capability>()?["enum"].clone();
    files.insert(
        "capabilities.json",
        json!({"schema_version":1,"capabilities":capabilities}),
    );
    let limits: BTreeMap<_, _> = WIRE_LIMITS.iter().copied().collect();
    files.insert("limits.json", json!({"schema_version":1,"wire":limits}));

    files
        .into_iter()
        .map(|(name, mut value)| {
            if name.ends_with(".schema.json") {
                value["x-chronlyt-semantic-validation-required"] = true.into();
            }
            let mut bytes = serde_json::to_vec_pretty(&value)?;
            bytes.push(b'\n');
            Ok((name.to_owned(), bytes))
        })
        .collect()
}
