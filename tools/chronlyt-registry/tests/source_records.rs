mod common;
use chronlyt_registry::RegistrySourceV1;
use serde_json::json;

#[test]
fn exact_source_format_has_no_author_controlled_integrity_or_trust() {
    common::source();
    for field in [
        "size",
        "sha256",
        "official",
        "verified",
        "downloads",
        "stars",
    ] {
        let mut value = common::record();
        value[field] = json!(1);
        assert!(
            RegistrySourceV1::from_blobs(
                &common::metadata(),
                &[(
                    "example.notes.json".into(),
                    serde_json::to_vec(&value).unwrap()
                )]
            )
            .is_err()
        );
        let mut value = common::record();
        value["release"][field] = json!(1);
        assert!(
            RegistrySourceV1::from_blobs(
                &common::metadata(),
                &[(
                    "example.notes.json".into(),
                    serde_json::to_vec(&value).unwrap()
                )]
            )
            .is_err()
        );
    }
}

#[test]
fn invalid_source_values_and_timestamps_fail() {
    for (field, invalid) in [
        ("schema_version", json!(2)),
        ("id", json!("../escape")),
        ("repository_url", json!("http://github.com/example/notes")),
        ("summary", json!("x".repeat(301))),
        ("summary", json!("")),
        ("categories", json!(["notes", "notes"])),
        ("tags", json!(["x".repeat(65)])),
        ("created_at", json!("2026-08-29T11:00:00Z")),
        ("updated_at", json!("2027-01-01T00:00:00Z")),
        ("updated_at", json!("not a date")),
    ] {
        let mut value = common::record();
        value[field] = invalid;
        assert!(
            RegistrySourceV1::from_blobs(
                &common::metadata(),
                &[(
                    "example.notes.json".into(),
                    serde_json::to_vec(&value).unwrap()
                )]
            )
            .is_err(),
            "{field}"
        );
    }
    for (field, value) in [
        ("version", "latest"),
        ("artifact_url", "https://evil.invalid/a.zip"),
    ] {
        let mut record = common::record();
        record["release"][field] = value.into();
        assert!(
            RegistrySourceV1::from_blobs(
                &common::metadata(),
                &[(
                    "example.notes.json".into(),
                    serde_json::to_vec(&record).unwrap()
                )]
            )
            .is_err()
        );
    }
}

#[test]
fn record_names_must_match_unique_plugin_ids() {
    let bytes = serde_json::to_vec(&common::record()).unwrap();
    for name in [
        "other.plugin.json",
        "../example.notes.json",
        "example.notes",
        "README.md",
        ".gitkeep",
    ] {
        assert!(
            RegistrySourceV1::from_blobs(&common::metadata(), &[(name.into(), bytes.clone())])
                .is_err()
        );
    }
    assert!(
        RegistrySourceV1::from_blobs(
            &common::metadata(),
            &[
                ("example.notes.json".into(), bytes.clone()),
                ("example.notes.json".into(), bytes)
            ]
        )
        .is_err()
    );
    RegistrySourceV1::from_blobs(&common::metadata(), &[(".gitkeep".into(), vec![])]).unwrap();
}
