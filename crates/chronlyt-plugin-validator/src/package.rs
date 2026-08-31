use std::{
    collections::BTreeSet,
    io::{Cursor, Read},
};

use chronlyt_plugin_contracts::{
    ContractError, PluginCatalogEntryV1, PluginManifestV1, limits::*, validate_relative_path,
    validate_sha256,
};
use sha2::{Digest, Sha256};
use zip::{CompressionMethod, ZipArchive};

use crate::{ValidationError, ValidationResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSummary {
    pub size: u64,
    pub sha256: String,
}

/// Constructible only after validation; no paths are written or executed here.
#[derive(Debug)]
pub struct ValidatedPackage {
    manifest: PluginManifestV1,
    files: Vec<(String, Vec<u8>)>,
    summary: PackageSummary,
}

impl ValidatedPackage {
    pub fn manifest(&self) -> &PluginManifestV1 {
        &self.manifest
    }
    pub fn summary(&self) -> &PackageSummary {
        &self.summary
    }
    pub fn component(&self) -> &[u8] {
        &self.files[1].1
    }
    pub fn files(&self) -> &[(String, Vec<u8>)] {
        &self.files
    }
}

/// Inspect archive structure and all internal digests, not the component ABI.
pub fn inspect_package(bytes: &[u8]) -> ValidationResult<ValidatedPackage> {
    check_compressed_size(bytes)?;
    let summary = PackageSummary {
        size: bytes.len() as u64,
        sha256: digest(bytes),
    };
    inspect(bytes, summary)
}

/// Enforce external integrity before opening the archive or allocating its files.
pub fn validate_package(
    bytes: &[u8],
    expected: &PluginCatalogEntryV1,
) -> ValidationResult<ValidatedPackage> {
    check_compressed_size(bytes)?;
    if bytes.len() as u64 != expected.artifact.size {
        return Err(ValidationError::Integrity);
    }
    validate_sha256(&expected.artifact.sha256)?;
    let actual = digest(bytes);
    if actual != expected.artifact.sha256 {
        return Err(ValidationError::Integrity);
    }
    let package = inspect(
        bytes,
        PackageSummary {
            size: bytes.len() as u64,
            sha256: actual,
        },
    )?;
    expected.verify_manifest(package.manifest())?;
    Ok(package)
}

fn check_compressed_size(bytes: &[u8]) -> ValidationResult<()> {
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(ContractError::ResourceLimit("compressed package bytes").into());
    }
    Ok(())
}

fn inspect(bytes: &[u8], summary: PackageSummary) -> ValidationResult<ValidatedPackage> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    check_directory_count(bytes, &archive)?;
    let entries = inspect_entries(&mut archive)?;
    let manifest_bytes = read_entry(&mut archive, "manifest.json", MAX_MANIFEST_BYTES as u64)?;
    let manifest = PluginManifestV1::parse(&manifest_bytes)?;
    let mut declared =
        BTreeSet::from(["manifest.json".to_owned(), manifest.component.path.clone()]);
    for asset in &manifest.assets {
        declared.insert(asset.path.clone());
    }
    if entries != declared {
        return Err(ValidationError::Archive);
    }
    let component = read_entry(
        &mut archive,
        &manifest.component.path,
        MAX_COMPONENT_BYTES as u64,
    )?;
    verify_digest(&component, &manifest.component.sha256)?;
    let mut files = vec![
        ("manifest.json".into(), manifest_bytes),
        (manifest.component.path.clone(), component),
    ];
    for asset in &manifest.assets {
        let contents = read_entry(&mut archive, &asset.path, MAX_ASSET_BYTES as u64)?;
        verify_digest(&contents, &asset.sha256)?;
        files.push((asset.path.clone(), contents));
    }
    Ok(ValidatedPackage {
        manifest,
        files,
        summary,
    })
}

// zip 8 indexes entries by raw filename and collapses duplicate central records.
// Count raw headers at its resolved directory offset before using that index.
// Decompression, ZIP64 resolution and metadata parsing remain the library's job.
fn check_directory_count(
    bytes: &[u8],
    archive: &ZipArchive<Cursor<&[u8]>>,
) -> ValidationResult<()> {
    let start =
        usize::try_from(archive.central_directory_start()).map_err(|_| ValidationError::Archive)?;
    let mut remaining = bytes.get(start..).ok_or(ValidationError::Archive)?;
    let mut count = 0;
    while remaining.starts_with(b"PK\x01\x02") {
        count += 1;
        if count > MAX_ARCHIVE_ENTRIES || count > archive.len() {
            return Err(ValidationError::Archive);
        }
        let header = remaining.get(..46).ok_or(ValidationError::Archive)?;
        let length = |at| u16::from_le_bytes([header[at], header[at + 1]]) as usize;
        let size = 46 + length(28) + length(30) + length(32);
        remaining = remaining.get(size..).ok_or(ValidationError::Archive)?;
    }
    if count != archive.len() {
        return Err(ValidationError::Archive);
    }
    Ok(())
}

fn inspect_entries(archive: &mut ZipArchive<Cursor<&[u8]>>) -> ValidationResult<BTreeSet<String>> {
    if archive.is_empty() || archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(ContractError::ResourceLimit("archive entries").into());
    }
    let mut entries = BTreeSet::new();
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = entry.name().to_owned();
        validate_relative_path(&name)?;
        if entry.is_dir() || entry.encrypted() {
            return Err(ValidationError::Archive);
        }
        if !matches!(
            entry.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(ValidationError::Archive);
        }
        if entry.unix_mode().is_some_and(|mode| {
            let file_type = mode & 0o170000;
            file_type != 0 && file_type != 0o100000
        }) {
            return Err(ValidationError::Archive);
        }
        total = total
            .checked_add(entry.size())
            .ok_or(ContractError::ResourceLimit("uncompressed package bytes"))?;
        if total > MAX_UNCOMPRESSED_BYTES {
            return Err(ContractError::ResourceLimit("uncompressed package bytes").into());
        }
        if !entries.insert(name) {
            return Err(ValidationError::Archive);
        }
    }
    Ok(entries)
}

fn read_entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    path: &str,
    max_bytes: u64,
) -> ValidationResult<Vec<u8>> {
    let mut entry = archive.by_name(path)?;
    if entry.size() > max_bytes {
        return Err(ContractError::ResourceLimit("package entry bytes").into());
    }
    let capacity = usize::try_from(entry.size())
        .map_err(|_| ContractError::ResourceLimit("package entry bytes"))?;
    let mut bytes = Vec::with_capacity(capacity);
    entry.by_ref().take(max_bytes + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Err(ContractError::ResourceLimit("package entry bytes").into());
    }
    Ok(bytes)
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn verify_digest(bytes: &[u8], expected: &str) -> ValidationResult<()> {
    if digest(bytes) == expected {
        Ok(())
    } else {
        Err(ValidationError::Integrity)
    }
}
