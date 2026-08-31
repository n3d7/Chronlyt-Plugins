use chronlyt_plugin_contracts::limits::*;
use chronlyt_registry::schema_export::generated_contracts;
use serde_json::{Value, json};

#[test]
fn schemas_preserve_closed_shapes_defaults_and_nullable_patch_inputs() {
    let files = generated_contracts().unwrap();
    let schema = |name: &str| -> Value { serde_json::from_slice(&files[name]).unwrap() };
    let manifest = schema("manifest.schema.json");
    assert_eq!(manifest["additionalProperties"], false);
    assert_eq!(manifest["properties"]["schema_version"]["const"], 1);
    assert_eq!(
        manifest["properties"]["name"]["x-chronlyt-max-utf8-bytes"],
        MAX_NAME_BYTES
    );
    assert!(manifest["properties"]["name"].get("maxLength").is_none());
    let payloads = schema("host-payloads.schema.json");
    let patch = &payloads["$defs"]["UpdateTimelineEntryInput"];
    assert_eq!(patch["additionalProperties"], false);
    assert!(
        !patch["required"]
            .as_array()
            .is_some_and(|required| required.contains(&json!("duration_seconds")))
    );
    assert_eq!(
        patch["properties"]["duration_seconds"]["type"],
        json!(["integer", "null"])
    );
    assert_eq!(
        patch["properties"]["duration_seconds"]["maximum"],
        MAX_DURATION_SECONDS
    );
    assert_ne!(
        payloads["$defs"]["TimelineEntry"]["additionalProperties"],
        false
    );
    assert_eq!(
        payloads["$defs"]["CreatePluginTimelineEntryInput"]["properties"]["note"]["default"],
        ""
    );
    let ui = schema("declarative-ui.schema.json");
    assert_eq!(ui["x-chronlyt-max-depth"], MAX_UI_DEPTH);
    let nodes = ui["$defs"]["PluginNodeV1"]["oneOf"].as_array().unwrap();
    assert!(
        nodes
            .iter()
            .all(|node| node["additionalProperties"] == false)
    );
    let button = nodes
        .iter()
        .find(|node| node["properties"]["kind"]["const"] == "button")
        .unwrap();
    assert_eq!(button["properties"]["disabled"]["default"], false);
    let registry = schema("registry-source.schema.json");
    assert_eq!(registry["additionalProperties"], false);
    for excluded in ["size", "sha256", "verified", "official"] {
        assert!(registry["properties"].get(excluded).is_none());
    }
    assert!(registry["$defs"].get("RegistryMetadataV1").is_some());
}

#[test]
fn generated_limits_capabilities_and_bytes_come_from_the_canonical_crate() {
    let files = generated_contracts().unwrap();
    assert_eq!(files, generated_contracts().unwrap());
    assert_eq!(files.len(), 7);
    assert!(
        files
            .values()
            .all(|bytes| bytes.ends_with(b"\n") && !bytes.ends_with(b"\n\n"))
    );
    let limits: Value = serde_json::from_slice(&files["limits.json"]).unwrap();
    assert_eq!(limits["wire"]["MAX_PACKAGE_BYTES"], MAX_PACKAGE_BYTES);
    assert_eq!(limits["wire"]["MAX_UI_DEPTH"], MAX_UI_DEPTH);
    let capabilities: Value = serde_json::from_slice(&files["capabilities.json"]).unwrap();
    for id in capabilities["capabilities"].as_array().unwrap() {
        chronlyt_plugin_contracts::Capability::from_id(id.as_str().unwrap()).unwrap();
    }
    assert_eq!(capabilities["capabilities"].as_array().unwrap().len(), 5);
}
