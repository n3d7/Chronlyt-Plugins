//! Registry acquisition only: no contribution checkout, filters, hooks, Cargo
//! invocation or executable artifacts. PR acquisition is a separate HTTP path.
mod api;
use crate::{
    RegistryError, RegistryResult, RegistrySourceV1,
    source::{MAX_METADATA_BYTES, MAX_RECORD_BYTES, MAX_SOURCE_BYTES, record_identity},
};
pub use api::{PrDataReader, PrDataSnapshot, read_source_pr};
use chronlyt_plugin_contracts::limits::{MAX_CATALOG_BYTES, MAX_CATALOG_ENTRIES};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Read,
    path::{Component, Path},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const MAX_TREE_BYTES: usize = 1024 * 1024;
const MAX_REQUESTS: usize = 2_100;
const ACQUISITION_TIMEOUT: Duration = Duration::from_secs(600);

pub fn validate_revision(value: &str) -> RegistryResult<()> {
    if value.len() != 40
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(RegistryError::Invalid("full revision required"));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub(crate) struct TreeEntry {
    name: String,
    mode: String,
    kind: String,
    sha: String,
    size: Option<u64>,
}

pub(crate) trait BlobReader {
    fn tree(&mut self, revision: &str) -> RegistryResult<Vec<TreeEntry>>;
    fn blob(&mut self, entry: &TreeEntry, max_bytes: usize) -> RegistryResult<Vec<u8>>;
}

pub struct GitDataSnapshot {
    pub source: RegistrySourceV1,
    pub catalog_bytes: Vec<u8>,
}

fn named<'a>(entries: &'a [TreeEntry], name: &str) -> RegistryResult<&'a TreeEntry> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .ok_or(RegistryError::Invalid("required registry path missing"))
}
fn regular(entry: &TreeEntry) -> RegistryResult<()> {
    validate_revision(&entry.sha)?;
    if entry.mode != "100644" || entry.kind != "blob" || entry.size.is_none() {
        return Err(RegistryError::Invalid("registry blob mode"));
    }
    Ok(())
}
fn directory(entry: &TreeEntry) -> RegistryResult<()> {
    validate_revision(&entry.sha)?;
    if entry.mode != "040000" || entry.kind != "tree" {
        return Err(RegistryError::Invalid("registry tree mode"));
    }
    Ok(())
}
fn validate_tree(entries: &[TreeEntry]) -> RegistryResult<()> {
    let mut names = BTreeSet::new();
    for entry in entries {
        validate_revision(&entry.sha)?;
        if entry.name.is_empty()
            || entry.name.contains(['/', '\\', '\0'])
            || matches!(entry.name.as_str(), "." | "..")
            || !names.insert(&entry.name)
        {
            return Err(RegistryError::Invalid("tree entry name"));
        }
    }
    Ok(())
}

fn source_from_tree(
    reader: &mut dyn BlobReader,
    root: &[TreeEntry],
) -> RegistryResult<RegistrySourceV1> {
    validate_tree(root)?;
    let registry = named(root, "registry")?;
    directory(registry)?;
    let entries = reader.tree(&registry.sha)?;
    validate_tree(&entries)?;
    if entries.len() != 2 {
        return Err(RegistryError::Invalid("unexpected registry path"));
    }
    let metadata = named(&entries, "metadata.json")?;
    regular(metadata)?;
    let plugins = named(&entries, "plugins")?;
    directory(plugins)?;
    let metadata = reader.blob(metadata, MAX_METADATA_BYTES)?;
    let entries = reader.tree(&plugins.sha)?;
    validate_tree(&entries)?;
    if entries.len() > MAX_CATALOG_ENTRIES + 1 {
        return Err(RegistryError::Limit("source records"));
    }
    let mut blobs = Vec::new();
    for entry in entries {
        regular(&entry)?;
        let cap = if record_identity(&entry.name)?.is_some() {
            MAX_RECORD_BYTES
        } else {
            0
        };
        let bytes = reader.blob(&entry, cap)?;
        blobs.push((entry.name, bytes));
    }
    RegistrySourceV1::from_blobs(&metadata, &blobs)
}

fn catalog_from_tree(
    reader: &mut dyn BlobReader,
    root: &[TreeEntry],
) -> RegistryResult<Option<Vec<u8>>> {
    validate_tree(root)?;
    let Some(entry) = root.iter().find(|entry| entry.name == "catalog.json") else {
        return Ok(None);
    };
    regular(entry)?;
    Ok(Some(reader.blob(entry, MAX_CATALOG_BYTES)?))
}

pub fn read_source_git(repo: &Path, revision: &str) -> RegistryResult<RegistrySourceV1> {
    validate_revision(revision)?;
    let mut reader = LocalGit::new(repo);
    let root = reader.tree(revision)?;
    source_from_tree(&mut reader, &root)
}

pub fn read_snapshot_git(repo: &Path, revision: &str) -> RegistryResult<GitDataSnapshot> {
    validate_revision(revision)?;
    let mut reader = LocalGit::new(repo);
    let root = reader.tree(revision)?;
    let catalog_bytes =
        catalog_from_tree(&mut reader, &root)?.ok_or(RegistryError::Invalid("catalog missing"))?;
    let source = source_from_tree(&mut reader, &root)?;
    Ok(GitDataSnapshot {
        source,
        catalog_bytes,
    })
}

/// None distinguishes an absent bootstrap catalog from a corrupt/unsafe one.
/// The command layer must allow this only for an empty initial registry.
pub fn read_catalog_git(repo: &Path, revision: &str) -> RegistryResult<Option<Vec<u8>>> {
    validate_revision(revision)?;
    let mut reader = LocalGit::new(repo);
    let root = reader.tree(revision)?;
    catalog_from_tree(&mut reader, &root)
}

struct LocalGit<'a> {
    repo: &'a Path,
    deadline: Instant,
    requests: usize,
    bytes: usize,
}
impl<'a> LocalGit<'a> {
    fn new(repo: &'a Path) -> Self {
        Self {
            repo,
            deadline: Instant::now() + ACQUISITION_TIMEOUT,
            requests: 0,
            bytes: 0,
        }
    }
    fn run(&mut self, args: &[&str], cap: usize) -> RegistryResult<Vec<u8>> {
        self.requests += 1;
        if self.requests > MAX_REQUESTS {
            return Err(RegistryError::Limit("Git requests"));
        }
        run_git(
            self.repo,
            args,
            cap,
            self.deadline.min(Instant::now() + Duration::from_secs(30)),
        )
    }
}

impl BlobReader for LocalGit<'_> {
    fn tree(&mut self, revision: &str) -> RegistryResult<Vec<TreeEntry>> {
        validate_revision(revision)?;
        let bytes = self.run(&["ls-tree", "-l", "-z", revision], MAX_TREE_BYTES)?;
        let mut entries = Vec::new();
        for record in bytes.split(|b| *b == 0).filter(|record| !record.is_empty()) {
            let record =
                std::str::from_utf8(record).map_err(|_| RegistryError::Invalid("tree encoding"))?;
            let (header, name) = record
                .split_once('\t')
                .ok_or(RegistryError::Invalid("tree encoding"))?;
            let fields: Vec<_> = header.split_ascii_whitespace().collect();
            if fields.len() != 4 {
                return Err(RegistryError::Invalid("tree encoding"));
            }
            let size = if fields[3] == "-" {
                None
            } else {
                Some(
                    fields[3]
                        .parse::<u64>()
                        .map_err(|_| RegistryError::Invalid("blob size"))?,
                )
            };
            entries.push(TreeEntry {
                name: name.into(),
                mode: fields[0].into(),
                kind: fields[1].into(),
                sha: fields[2].into(),
                size,
            });
        }
        validate_tree(&entries)?;
        Ok(entries)
    }
    fn blob(&mut self, entry: &TreeEntry, max_bytes: usize) -> RegistryResult<Vec<u8>> {
        regular(entry)?;
        let cap = max_bytes.min(MAX_SOURCE_BYTES.saturating_sub(self.bytes));
        if entry.size.is_none_or(|size| size > cap as u64) {
            return Err(RegistryError::Limit("blob bytes"));
        }
        let bytes = self.run(&["cat-file", "blob", &entry.sha], cap)?;
        if Some(bytes.len() as u64) != entry.size {
            return Err(RegistryError::Invalid("blob size disagreement"));
        }
        self.bytes += bytes.len();
        Ok(bytes)
    }
}

fn run_git(repo: &Path, args: &[&str], cap: usize, deadline: Instant) -> RegistryResult<Vec<u8>> {
    if Instant::now() >= deadline {
        return Err(RegistryError::Limit("Git deadline"));
    }
    let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let mut command = Command::new("git");
    command.env_clear();
    if let Some(path) = std::env::var_os("PATH") {
        command.env("PATH", path);
    }
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", null)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .arg("--no-pager")
        .arg("--no-replace-objects")
        .arg("--no-lazy-fetch")
        .args(["-c", "core.fsmonitor=false", "-c", "protocol.allow=never"])
        .arg("-c")
        .arg(format!("core.hooksPath={null}"))
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command.spawn().map_err(|_| RegistryError::Io)?;
    let stdout = child.stdout.take().ok_or(RegistryError::Io)?;
    let (send, receive) = mpsc::sync_channel(1);
    let reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout
            .take(cap as u64 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = send.send(result);
    });
    let received = receive.recv_timeout(deadline.saturating_duration_since(Instant::now()));
    let bytes = match received {
        Ok(Ok(bytes)) if bytes.len() <= cap => bytes,
        _ => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err(RegistryError::Limit("Git output or deadline"));
        }
    };
    let _ = reader.join();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(bytes),
            Ok(Some(_)) => return Err(RegistryError::Io),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(RegistryError::Limit("Git deadline"));
            }
        }
    }
}

fn ensure_directory(path: &Path) -> RegistryResult<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(RegistryError::Invalid("source directory"));
    }
    Ok(())
}
pub(crate) fn read_regular(path: &Path, cap: usize) -> RegistryResult<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(RegistryError::Invalid("source file"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 != 0 {
            return Err(RegistryError::Invalid("executable source file"));
        }
    }
    if metadata.len() > cap as u64 {
        return Err(RegistryError::Limit("source file bytes"));
    }
    let mut bytes = Vec::new();
    File::open(path)?
        .take(cap as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > cap || bytes.len() as u64 != metadata.len() {
        return Err(RegistryError::Limit("source file bytes"));
    }
    Ok(bytes)
}

pub fn read_source_directory(root: &Path) -> RegistryResult<RegistrySourceV1> {
    let mut prefix = std::path::PathBuf::new();
    for component in root.components() {
        if component == Component::ParentDir {
            return Err(RegistryError::Invalid("source directory traversal"));
        }
        prefix.push(component);
        ensure_directory(&prefix)?;
    }
    ensure_directory(root)?;
    let registry = root.join("registry");
    ensure_directory(&registry)?;
    let mut paths = BTreeSet::new();
    for entry in fs::read_dir(&registry)? {
        let name = entry?.file_name();
        let name = name
            .to_str()
            .ok_or(RegistryError::Invalid("source filename"))?;
        if !matches!(name, "metadata.json" | "plugins") {
            return Err(RegistryError::Invalid("unexpected registry path"));
        }
        paths.insert(name.to_owned());
    }
    if paths.len() != 2 {
        return Err(RegistryError::Invalid("required registry path missing"));
    }
    let plugins = registry.join("plugins");
    ensure_directory(&plugins)?;
    let metadata = read_regular(&registry.join("metadata.json"), MAX_METADATA_BYTES)?;
    let mut total = metadata.len();
    let mut blobs = Vec::new();
    for entry in fs::read_dir(plugins)? {
        if blobs.len() > MAX_CATALOG_ENTRIES {
            return Err(RegistryError::Limit("source records"));
        }
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| RegistryError::Invalid("source filename"))?;
        let cap = if record_identity(&name)?.is_some() {
            MAX_RECORD_BYTES
        } else {
            0
        };
        let bytes = read_regular(
            &entry.path(),
            cap.min(MAX_SOURCE_BYTES.saturating_sub(total)),
        )?;
        total += bytes.len();
        blobs.push((name, bytes));
    }
    RegistrySourceV1::from_blobs(&metadata, &blobs)
}
