#![allow(dead_code)]

#[path = "../../../../crates/chronlyt-plugin-validator/tests/common/mod.rs"]
mod package_fixture;

use chronlyt_plugin_contracts::PluginCatalogV1;
use chronlyt_registry::{ArtifactFetcher, FetchBudget, RegistryResult, RegistrySourceV1};
use serde_json::{Value, json};

pub fn record() -> Value {
    json!({
        "schema_version":1, "id":"example.notes",
        "release":{"version":"1.0.0", "artifact_url":"https://github.com/example/notes/releases/download/v1.0.0/notes.chronlyt-plugin"},
        "repository_url":"https://github.com/example/notes", "summary":"Private notes",
        "publisher":"Example", "license":"MIT", "categories":["writing"], "tags":["notes"],
        "created_at":"2026-08-29T09:00:00Z", "updated_at":"2026-08-29T10:00:00Z", "changelog":"Initial release."
    })
}

pub fn metadata() -> Vec<u8> {
    serde_json::to_vec(&json!({"schema_version":1,"generated_at":"2026-08-30T00:00:00Z"})).unwrap()
}

pub fn source() -> RegistrySourceV1 {
    RegistrySourceV1::from_blobs(
        &metadata(),
        &[(
            "example.notes.json".into(),
            serde_json::to_vec(&record()).unwrap(),
        )],
    )
    .unwrap()
}

pub fn previous_catalog() -> PluginCatalogV1 {
    PluginCatalogV1::parse(include_bytes!("../../../../catalog.json")).unwrap()
}

pub fn package(id: &str, version: &str, description: &str) -> Vec<u8> {
    package_with_compatibility(id, version, description, ">=0.2.0")
}

pub fn package_with_compatibility(
    id: &str,
    version: &str,
    description: &str,
    compatibility: &str,
) -> Vec<u8> {
    let component =
        wat::parse_str(include_str!("../../../../fixtures/v1/component/valid.wat")).unwrap();
    let mut manifest = package_fixture::manifest();
    manifest["id"] = id.into();
    manifest["version"] = version.into();
    manifest["description"] = description.into();
    manifest["compatibility"]["chronlyt"] = compatibility.into();
    manifest["component"]["sha256"] = package_fixture::digest(&component).into();
    package_fixture::zip(&[
        ("manifest.json", &serde_json::to_vec(&manifest).unwrap()),
        ("plugin.wasm", &component),
    ])
}

pub struct MemoryFetcher {
    pub bytes: Vec<u8>,
    pub calls: usize,
}
impl MemoryFetcher {
    pub fn valid() -> Self {
        Self {
            bytes: package("example.notes", "1.0.0", "A bounded notes page."),
            calls: 0,
        }
    }
}
impl ArtifactFetcher for MemoryFetcher {
    fn fetch(&mut self, _: &str, _: FetchBudget) -> RegistryResult<Vec<u8>> {
        self.calls += 1;
        Ok(self.bytes.clone())
    }
}
