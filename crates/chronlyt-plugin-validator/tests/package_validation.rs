mod common;

use chronlyt_plugin_validator::{inspect_package, validate_package};
use common::*;
use serde_json::json;

#[test]
fn package_summary_is_computed_from_validated_bytes() {
    let bytes = package_bytes();
    assert_eq!(bytes, package_bytes());
    let package = inspect_package(&bytes).unwrap();
    assert_eq!(package.summary().size, bytes.len() as u64);
    assert_eq!(package.summary().sha256, digest(&bytes));
    assert_eq!(package.manifest().id.as_str(), "example.notes");
    assert_eq!(package.component(), COMPONENT);
    assert_eq!(package.files().len(), 2);
    validate_package(&bytes, &expected(&bytes)).unwrap();
}

#[test]
fn outer_integrity_and_catalog_manifest_match_are_required() {
    let bytes = package_bytes();
    let valid = expected(&bytes);
    let mut wrong = valid.clone();
    wrong.artifact.size += 1;
    assert!(validate_package(&bytes, &wrong).is_err());
    wrong = valid.clone();
    wrong.artifact.sha256 = "0".repeat(64);
    assert!(validate_package(&bytes, &wrong).is_err());
    wrong = valid.clone();
    wrong.version = "2.0.0".into();
    assert!(validate_package(&bytes, &wrong).is_err());
    wrong = valid;
    wrong.permissions.clear();
    assert!(validate_package(&bytes, &wrong).is_err());
}

#[test]
fn component_and_asset_hashes_are_verified() {
    let mut manifest = manifest();
    let bad_component = zip(&[
        ("manifest.json", &serde_json::to_vec(&manifest).unwrap()),
        ("plugin.wasm", b"tampered"),
    ]);
    assert!(inspect_package(&bad_component).is_err());
    manifest["assets"] = json!([{
        "path":"assets/note.txt", "sha256":digest(b"note"), "media_type":"image/png"
    }]);
    for (asset, accepted) in [(b"note".as_slice(), true), (b"tampered".as_slice(), false)] {
        let bytes = zip(&[
            ("manifest.json", &serde_json::to_vec(&manifest).unwrap()),
            ("plugin.wasm", COMPONENT),
            ("assets/note.txt", asset),
        ]);
        let result = inspect_package(&bytes);
        assert_eq!(result.is_ok(), accepted, "{result:?}");
    }
}

#[test]
fn paths_and_exact_declared_file_set_are_enforced() {
    let manifest = serde_json::to_vec(&manifest()).unwrap();
    for path in [
        "../escape",
        "/absolute",
        "a\\b",
        "undeclared.txt",
        "folder/",
    ] {
        let bytes = zip(&[
            ("manifest.json", &manifest),
            ("plugin.wasm", COMPONENT),
            (path, b"x"),
        ]);
        assert!(inspect_package(&bytes).is_err(), "{path}");
    }
    assert!(inspect_package(&zip(&[("manifest.json", &manifest)])).is_err());
    assert!(inspect_package(&zip(&[])).is_err());
}

#[test]
fn archive_metadata_cannot_bypass_file_safety() {
    let bytes = package_bytes();
    let header = central_headers(&bytes)[1];
    // Unix symlink/device, encryption and unsupported compression are all rejected.
    for mode in [0o120777u32, 0o020600] {
        let mut bad = bytes.clone();
        bad[header + 5] = 3; // Unix made-by platform
        bad[header + 38..header + 42].copy_from_slice(&(mode << 16).to_le_bytes());
        assert!(inspect_package(&bad).is_err());
    }
    let mut bad = bytes.clone();
    bad[header + 8] |= 1;
    assert!(inspect_package(&bad).is_err());
    let mut bad = bytes;
    bad[header + 10..header + 12].copy_from_slice(&12u16.to_le_bytes());
    assert!(inspect_package(&bad).is_err());
}

#[test]
fn duplicate_names_and_truncated_archives_are_rejected() {
    let mut bytes = zip(&[
        ("manifest.json", &serde_json::to_vec(&manifest()).unwrap()),
        ("plugin.wasm", COMPONENT),
        ("plugi2.wasm", COMPONENT),
    ]);
    let headers = central_headers(&bytes);
    let extra = headers[2];
    bytes[extra + 46..extra + 57].copy_from_slice(b"plugin.wasm");
    let local = u32::from_le_bytes(bytes[extra + 42..extra + 46].try_into().unwrap()) as usize;
    bytes[local + 30..local + 41].copy_from_slice(b"plugin.wasm");
    assert!(inspect_package(&bytes).is_err());
    let bytes = package_bytes();
    for length in [0, 4, bytes.len() - 1, bytes.len() / 2] {
        assert!(inspect_package(&bytes[..length]).is_err());
    }
}

#[test]
fn declared_and_actual_size_limits_are_checked() {
    assert!(inspect_package(&vec![0; 16 * 1024 * 1024 + 1]).is_err());
    let manifest = serde_json::to_vec(&manifest()).unwrap();
    let names: Vec<_> = (0..255).map(|i| format!("extra-{i}.txt")).collect();
    let mut entries = vec![
        ("manifest.json", manifest.as_slice()),
        ("plugin.wasm", COMPONENT),
    ];
    entries.extend(names.iter().map(|name| (name.as_str(), b"x".as_slice())));
    assert!(inspect_package(&zip(&entries)).is_err());
    let bytes = package_bytes();
    for (index, size) in [(0, 128 * 1024 + 1), (1, 32 * 1024 * 1024 + 1)] {
        let mut bad = bytes.clone();
        let h = central_headers(&bad)[index];
        bad[h + 24..h + 28].copy_from_slice(&(size as u32).to_le_bytes());
        assert!(inspect_package(&bad).is_err());
    }
    let mut large_manifest = manifest;
    large_manifest.resize(128 * 1024 + 1, b' ');
    assert!(
        inspect_package(&zip(&[
            ("manifest.json", &large_manifest),
            ("plugin.wasm", COMPONENT)
        ]))
        .is_err()
    );
}

#[test]
fn individual_asset_and_aggregate_uncompressed_limits_are_enforced() {
    let mut value = manifest();
    let asset = vec![b'x'; 4 * 1024 * 1024 + 1];
    value["assets"] =
        json!([{"path":"asset.png", "sha256":digest(&asset), "media_type":"image/png"}]);
    let bytes = zip(&[
        ("manifest.json", &serde_json::to_vec(&value).unwrap()),
        ("plugin.wasm", COMPONENT),
        ("asset.png", &asset),
    ]);
    assert!(inspect_package(&bytes).is_err());

    // Total declared data can be excessive even with a tiny compressed input.
    let mut bytes = zip(&[
        ("manifest.json", &serde_json::to_vec(&value).unwrap()),
        ("plugin.wasm", COMPONENT),
        ("asset.png", b"x"),
    ]);
    for header in central_headers(&bytes) {
        bytes[header + 24..header + 28].copy_from_slice(&(32u32 * 1024 * 1024).to_le_bytes());
    }
    let error = inspect_package(&bytes).unwrap_err();
    assert!(error.to_string().contains("uncompressed package bytes"));
}

#[test]
fn archive_errors_do_not_echo_untrusted_names() {
    let bytes = zip(&[("SECRET-UNTRUSTED-MARKER", b"not a plugin")]);
    let error = inspect_package(&bytes).unwrap_err();
    assert!(!format!("{error:?} {error}").contains("SECRET-UNTRUSTED-MARKER"));
}
