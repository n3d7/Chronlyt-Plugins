use std::{
    collections::BTreeSet,
    env, fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
};

use chronlyt_plugin_contracts::{
    PluginCompatibilityV1, PluginComponentV1, PluginId, PluginManifestV1, PluginPageManifestV1,
    PluginViewV1,
};
use chronlyt_plugin_validator::{inspect_package, validate_component_contract};
use sha2::{Digest, Sha256};
use wasmparser::{Parser, Payload};
use wit_component::ComponentEncoder;
use zip::{CompressionMethod, DateTime, ZipWriter, write::SimpleFileOptions};

#[test]
#[ignore = "requires the separately source-built canary core Wasm module"]
fn source_built_canary_is_a_valid_package() -> Result<(), Box<dyn std::error::Error>> {
    let core_path = env::var_os("CHRONLYT_CANARY_CORE_WASM")
        .map(PathBuf::from)
        .ok_or("CHRONLYT_CANARY_CORE_WASM must select the source-built core module")?;
    let core_module = fs::read(&core_path)?;
    assert!(!Parser::is_component(&core_module));

    let component = componentize(&core_module)?;
    assert!(Parser::is_component(&component));
    assert_no_foreign_outer_imports(&component)?;
    validate_component_contract(&component)?;

    let manifest = manifest_for(&component)?;
    let manifest_bytes = serde_json::to_vec(&manifest)?;
    let package_bytes = package(&manifest_bytes, &component)?;
    let inspected = inspect_package(&package_bytes)?;

    assert_eq!(inspected.manifest(), &manifest);
    assert_eq!(inspected.component(), component);
    assert_eq!(
        inspected
            .files()
            .iter()
            .map(|(path, _)| path.as_str())
            .collect::<Vec<_>>(),
        ["manifest.json", "plugin.wasm"]
    );
    validate_component_contract(inspected.component())?;

    for fixture in [
        include_bytes!("../../../fixtures/v1/canary/ready.json").as_slice(),
        include_bytes!("../../../fixtures/v1/canary/pong.json").as_slice(),
    ] {
        PluginViewV1::parse_and_validate(fixture)?;
    }

    let output_dir = output_dir();
    fs::create_dir_all(&output_dir)?;
    fs::write(output_dir.join("plugin.wasm"), &component)?;
    fs::write(
        output_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    fs::write(
        output_dir.join("minimal-page.chronlyt-plugin"),
        &package_bytes,
    )?;
    Ok(())
}

fn componentize(core_module: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut encoder = ComponentEncoder::default();
    encoder.module(core_module)?.validate(true);
    Ok(encoder.encode()?)
}

fn assert_no_foreign_outer_imports(component: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut depth = 0_u32;
    let mut imports = BTreeSet::new();
    for payload in Parser::new(0).parse_all(component) {
        match payload? {
            Payload::Version { .. } => depth += 1,
            Payload::End(_) => depth -= 1,
            Payload::ComponentImportSection(section) if depth == 1 => {
                for import in section {
                    imports.insert(import?.name.0.to_owned());
                }
            }
            _ => {}
        }
    }
    assert!(
        imports
            .iter()
            .all(|name| name == "chronlyt:plugin/host@1.0.0"),
        "foreign outer component imports: {imports:?}"
    );
    Ok(())
}

fn manifest_for(component: &[u8]) -> Result<PluginManifestV1, Box<dyn std::error::Error>> {
    let manifest = PluginManifestV1 {
        schema_version: 1,
        api_version: 1,
        id: PluginId::parse("example.minimal-page")?,
        name: "Minimal compatibility canary".into(),
        version: "0.1.0".into(),
        description: "Source-built compatibility canary; not a published plugin.".into(),
        author: "Chronlyt".into(),
        compatibility: PluginCompatibilityV1 {
            chronlyt: ">=0.2.1, <0.3.0".into(),
        },
        component: PluginComponentV1 {
            path: "plugin.wasm".into(),
            sha256: digest(component),
        },
        permissions: Vec::new(),
        pages: vec![PluginPageManifestV1 {
            id: "main".into(),
            title: "Compatibility canary".into(),
            description: "Minimal no-permission compatibility page.".into(),
        }],
        assets: Vec::new(),
    };
    let encoded = serde_json::to_vec(&manifest)?;
    Ok(PluginManifestV1::parse(&encoded)?)
}

fn package(manifest: &[u8], component: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0)?)
        .unix_permissions(0o644);
    for (path, bytes) in [("manifest.json", manifest), ("plugin.wasm", component)] {
        archive.start_file(path, options)?;
        archive.write_all(bytes)?;
    }
    Ok(archive.finish()?.into_inner())
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn output_dir() -> PathBuf {
    env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root().join("target"))
        .join("canary")
}

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("validator crate must be inside the public workspace")
}
