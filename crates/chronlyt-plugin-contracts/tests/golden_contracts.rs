use chronlyt_plugin_contracts::{
    CreatePluginTimelineEntryInput, PluginCatalogV1, PluginManifestV1, PluginStorageEntry,
    PluginViewV1, TimelineListQuery, UpdateTimelineEntryInput,
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    name: String,
    kind: String,
    file: String,
    accepted: bool,
    normalized: Option<String>,
}

#[test]
fn frozen_wire_cases_match_the_host_baseline() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/v1");
    let cases: Vec<Case> =
        serde_json::from_slice(&std::fs::read(root.join("cases.json")).unwrap()).unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let bytes = std::fs::read(root.join(&case.file)).unwrap();
        let result: Result<Value, String> = match case.kind.as_str() {
            "manifest" => PluginManifestV1::parse(&bytes)
                .map(|v| serde_json::to_value(v).unwrap())
                .map_err(|e| e.to_string()),
            "catalog" => PluginCatalogV1::parse(&bytes)
                .map(|v| serde_json::to_value(v).unwrap())
                .map_err(|e| e.to_string()),
            "ui" => PluginViewV1::parse_and_validate(&bytes)
                .map(|v| serde_json::to_value(v).unwrap())
                .map_err(|e| e.to_string()),
            "timeline_create" => serde_json::from_slice::<CreatePluginTimelineEntryInput>(&bytes)
                .map(|v| serde_json::to_value(v).unwrap())
                .map_err(|e| e.to_string()),
            "timeline_patch" => serde_json::from_slice::<UpdateTimelineEntryInput>(&bytes)
                .map(|v| serde_json::to_value(v).unwrap())
                .map_err(|e| e.to_string()),
            "timeline_query" => serde_json::from_slice::<TimelineListQuery>(&bytes)
                .map(|v| serde_json::to_value(v).unwrap())
                .map_err(|e| e.to_string()),
            "storage_entry" => serde_json::from_slice::<PluginStorageEntry>(&bytes)
                .map(|v| serde_json::to_value(v).unwrap())
                .map_err(|e| e.to_string()),
            kind => panic!("unsupported fixture kind: {kind}"),
        };
        assert_eq!(result.is_ok(), case.accepted, "{}: {result:?}", case.name);
        if let Some(normalized) = &case.normalized {
            let expected: Value =
                serde_json::from_slice(&std::fs::read(root.join(normalized)).unwrap()).unwrap();
            assert_eq!(result.unwrap(), expected, "{}", case.name);
        }
    }
}

#[test]
fn timeline_patch_preserves_missing_null_and_value() {
    let missing: UpdateTimelineEntryInput = serde_json::from_str("{}").unwrap();
    let cleared: UpdateTimelineEntryInput =
        serde_json::from_str(r#"{"duration_seconds":null}"#).unwrap();
    let set: UpdateTimelineEntryInput = serde_json::from_str(r#"{"duration_seconds":30}"#).unwrap();
    assert_eq!(missing.duration_seconds, None);
    assert_eq!(cleared.duration_seconds, Some(None));
    assert_eq!(set.duration_seconds, Some(Some(30)));
}
