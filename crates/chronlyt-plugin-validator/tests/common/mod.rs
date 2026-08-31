#![allow(dead_code)]

use chronlyt_plugin_contracts::{PluginCatalogEntryV1, PluginCatalogV1};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::{Cursor, Write};
use zip::{CompressionMethod, DateTime, ZipWriter, write::SimpleFileOptions};

pub const COMPONENT: &[u8] = b"\0asm\x0d\0\x01\0";

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn manifest() -> Value {
    let mut manifest: Value =
        serde_json::from_str(include_str!("../../../../fixtures/v1/manifest/valid.json")).unwrap();
    manifest["component"]["sha256"] = digest(COMPONENT).into();
    manifest
}

pub fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).unwrap());
    for (path, bytes) in entries {
        archive.start_file(*path, options).unwrap();
        archive.write_all(bytes).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

pub fn package_bytes() -> Vec<u8> {
    zip(&[
        ("manifest.json", &serde_json::to_vec(&manifest()).unwrap()),
        ("plugin.wasm", COMPONENT),
    ])
}

pub fn expected(bytes: &[u8]) -> PluginCatalogEntryV1 {
    let mut value: Value =
        serde_json::from_str(include_str!("../../../../fixtures/v1/catalog/valid.json")).unwrap();
    value["plugins"][0]["artifact"]["sha256"] = digest(bytes).into();
    value["plugins"][0]["artifact"]["size"] = (bytes.len() as u64).into();
    PluginCatalogV1::parse(&serde_json::to_vec(&value).unwrap())
        .unwrap()
        .plugins
        .remove(0)
}

// Locate central headers only inside the EOCD-delimited directory, never file data.
pub fn central_headers(bytes: &[u8]) -> Vec<usize> {
    let end = bytes.len() - 22;
    assert_eq!(&bytes[end..end + 4], b"PK\x05\x06");
    let count = u16::from_le_bytes(bytes[end + 10..end + 12].try_into().unwrap());
    let mut offset = u32::from_le_bytes(bytes[end + 16..end + 20].try_into().unwrap()) as usize;
    let mut headers = Vec::new();
    for _ in 0..count {
        assert_eq!(&bytes[offset..offset + 4], b"PK\x01\x02");
        headers.push(offset);
        let length = |at| u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap()) as usize;
        offset += 46 + length(offset + 28) + length(offset + 30) + length(offset + 32);
    }
    headers
}
