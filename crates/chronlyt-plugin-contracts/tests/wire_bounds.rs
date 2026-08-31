use chronlyt_plugin_contracts::{
    PluginId, PluginManifestV1, PluginViewV1, host_payloads::*, validate_artifact_url,
    validate_redirect_url, validate_relative_path,
};
use serde_json::json;

fn manifest() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../fixtures/v1/manifest/valid.json")).unwrap()
}

#[test]
fn byte_bounded_manifest_and_ascii_identifiers_are_preserved() {
    let mut value = manifest();
    value["name"] = json!("é".repeat(40));
    assert!(PluginManifestV1::parse(&serde_json::to_vec(&value).unwrap()).is_ok());
    value["name"] = json!("é".repeat(41));
    assert!(PluginManifestV1::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    for id in ["core", "UPPER.case", "example..notes", "notes-", "é", ""] {
        assert!(PluginId::parse(id).is_err(), "{id}");
    }
    assert!(PluginId::parse(&"a".repeat(57)).is_ok());
    assert!(PluginId::parse(&"a".repeat(58)).is_err());
    assert!(PluginManifestV1::parse(&vec![b' '; 128 * 1024 + 1]).is_err());
}

#[test]
fn closed_ui_checks_depth_children_nodes_and_utf8_bytes() {
    let parse = |root| {
        PluginViewV1::parse_and_validate(
            &serde_json::to_vec(&json!({"schema_version":1,"root":root})).unwrap(),
        )
    };
    assert!(parse(json!({"kind":"text","text":"é".repeat(2048)})).is_ok());
    assert!(parse(json!({"kind":"text","text":"é".repeat(2049)})).is_err());
    let mut root = json!({"kind":"divider"});
    for _ in 0..15 {
        root = json!({"kind":"stack","children":[root]});
    }
    assert!(parse(root.clone()).is_ok());
    assert!(parse(json!({"kind":"stack","children":[root]})).is_err());
    assert!(parse(json!({"kind":"row","children":vec![json!({"kind":"divider"});64]})).is_ok());
    assert!(parse(json!({"kind":"row","children":vec![json!({"kind":"divider"});65]})).is_err());
    let row = json!({"kind":"row","children":vec![json!({"kind":"divider"});63]});
    assert!(parse(json!({"kind":"page","children":vec![row;8]})).is_err());
}

#[test]
fn timeline_and_storage_retain_distinct_field_rules() {
    assert_eq!(validate_timeline_title("  hello  ").unwrap(), "hello");
    assert!(validate_timeline_title(&"界".repeat(200)).is_ok());
    assert!(validate_timeline_title(&"界".repeat(201)).is_err());
    assert!(validate_timeline_note(&"界".repeat(4000)).is_ok());
    assert!(validate_timeline_note(&"界".repeat(4001)).is_err());
    for duration in [None, Some(1), Some(31_536_000)] {
        assert!(validate_timeline_duration(duration).is_ok());
    }
    for duration in [Some(0), Some(-1), Some(31_536_001)] {
        assert!(validate_timeline_duration(duration).is_err());
    }
    assert!(validate_timeline_source_ref(Some(&"é".repeat(100))).is_ok());
    assert!(validate_timeline_source_ref(Some(&"é".repeat(101))).is_err());
    assert!(validate_storage_key(&"é".repeat(64)).is_ok());
    assert!(validate_storage_key(&"é".repeat(65)).is_err());
    assert!(validate_storage_key("x\0y").is_err());
    assert!(validate_storage_value("null").is_ok());
    assert!(validate_storage_value("not JSON").is_err());
    assert_eq!(
        normalize_timeline_timestamp("2026-08-30T01:00:00+01:00").unwrap(),
        "2026-08-30T00:00:00+00:00"
    );
    assert_eq!(validate_timeline_query_limit(None).unwrap(), 50);
    assert!(validate_timeline_query_limit(Some(0)).is_err());
    assert!(validate_timeline_query_limit(Some(101)).is_err());
}

#[test]
fn portable_paths_and_initial_versus_redirect_urls_remain_distinct() {
    for path in [
        "../plugin.wasm",
        "C:/plugin.wasm",
        "assets/CON.png",
        "assets/LPT1.png",
        "assets\\x.png",
    ] {
        assert!(validate_relative_path(path).is_err(), "{path}");
    }
    assert!(validate_relative_path("assets/icon.png").is_ok());
    let signed = "https://objects.githubusercontent.com/release/plugin?signature=opaque";
    assert!(validate_artifact_url(signed).is_err());
    assert!(validate_redirect_url(signed).is_ok());
    assert!(validate_redirect_url("https://github.com/release/plugin?signature=opaque").is_err());
    for url in [
        "http://github.com/x",
        "https://github.com.evil.example/x",
        "https://user:pass@github.com/x",
        "https://github.com:444/x",
        "https://github.com/x#fragment",
    ] {
        assert!(validate_artifact_url(url).is_err(), "{url}");
    }
}

#[test]
fn ui_enforces_total_inputs_nodes_and_control_field_bounds() {
    let parse = |root| {
        PluginViewV1::parse_and_validate(
            &serde_json::to_vec(&json!({"schema_version":1,"root":root})).unwrap(),
        )
    };
    let input = json!({"kind":"text_input","field_id":"note","label":"Note","value":""});
    let row = json!({"kind":"row","children":vec![input.clone();32]});
    let mut root = json!({"kind":"page","children":[row.clone(),row]});
    assert!(parse(root.clone()).is_ok());
    root["children"].as_array_mut().unwrap().push(input.clone());
    assert!(parse(root).is_err());
    let row = json!({"kind":"row","children":vec![json!({"kind":"divider"});63]});
    let mut root = json!({"kind":"page","children":vec![row;8]});
    root["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert!(parse(root.clone()).is_ok()); // 512 nodes, within all per-parent bounds.
    root["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"divider"}));
    assert!(parse(root).is_err()); // 513, not a per-parent overflow.
    let mut input = input;
    input["field_id"] = json!("a".repeat(64));
    assert!(parse(input.clone()).is_ok());
    input["field_id"] = json!("a".repeat(65));
    assert!(parse(input).is_err());
    for (length, accepted) in [(128, true), (129, false)] {
        assert_eq!(
            parse(json!({"kind":"button","label":"Go","action_id":"a".repeat(length)})).is_ok(),
            accepted
        );
    }
    for (length, accepted) in [(100, true), (101, false)] {
        assert_eq!(
            parse(json!({"kind":"list","items":vec![json!({"title":"X"});length]})).is_ok(),
            accepted
        );
        assert_eq!(parse(json!({"kind":"select","field_id":"pick","label":"Pick","value":"x","options":vec![json!({"value":"x","label":"X"});length]})).is_ok(), accepted);
    }
}

#[test]
fn manifest_declaration_counts_and_duplicate_assets_are_bounded() {
    for (count, accepted) in [(32, true), (33, false)] {
        let mut value = manifest();
        value["pages"] = json!(
            (0..count)
                .map(|i| json!({"id":format!("page{i}"),"title":"Page","description":""}))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            PluginManifestV1::parse(&serde_json::to_vec(&value).unwrap()).is_ok(),
            accepted
        );
    }
    for (count, accepted) in [(64, true), (65, false)] {
        let mut value = manifest();
        value["assets"] = json!((0..count).map(|i| json!({"path":format!("assets/icon{i}.png"),"sha256":"a".repeat(64),"media_type":"image/png"})).collect::<Vec<_>>());
        assert_eq!(
            PluginManifestV1::parse(&serde_json::to_vec(&value).unwrap()).is_ok(),
            accepted
        );
    }
    let mut value = manifest();
    let asset = json!({"path":"assets/x.png","sha256":"a".repeat(64),"media_type":"image/png"});
    value["assets"] = json!([asset.clone(), asset]);
    assert!(PluginManifestV1::parse(&serde_json::to_vec(&value).unwrap()).is_err());
}

#[test]
fn payload_limits_are_checked_on_valid_json_and_errors_are_bounded() {
    let at_limit = format!("\"{}\"", "a".repeat(65534));
    assert!(validate_storage_value(&at_limit).is_ok());
    assert!(validate_storage_value(&format!("{at_limit} ")).is_err());
    assert!(validate_timeline_record_id(&"a".repeat(128)).is_ok());
    assert!(validate_timeline_record_id(&"a".repeat(129)).is_err());
    assert!(validate_timeline_record_id("../entry").is_err());
    let mut bytes =
        serde_json::to_vec(&json!({"schema_version":1,"root":{"kind":"divider"}})).unwrap();
    bytes.resize(256 * 1024, b' ');
    assert!(PluginViewV1::parse_and_validate(&bytes).is_ok());
    bytes.push(b' ');
    assert!(PluginViewV1::parse_and_validate(&bytes).is_err());
    let mut value = manifest();
    value["private-input-marker"] = json!("not-for-diagnostics");
    let error = PluginManifestV1::parse(&serde_json::to_vec(&value).unwrap()).unwrap_err();
    assert!(!format!("{error:?}").contains("private-input-marker"));
    assert!(!error.to_string().contains("not-for-diagnostics"));
}
