mod common;
use chronlyt_registry::{read_source_directory, read_source_git};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};
use tempfile::TempDir;

fn git(repo: &Path, args: &[&str], input: &[u8]) -> String {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "git fixture setup failed");
    String::from_utf8(output.stdout).unwrap().trim().into()
}
fn blob(repo: &Path, bytes: &[u8]) -> String {
    git(repo, &["hash-object", "-w", "--stdin"], bytes)
}
fn tree(repo: &Path, entries: &str) -> String {
    git(repo, &["mktree"], entries.as_bytes())
}
fn fixture(mode: &str, name: &str) -> (TempDir, String) {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "--bare"], b"");
    let metadata = blob(root.path(), &common::metadata());
    let record = blob(root.path(), &serde_json::to_vec(&common::record()).unwrap());
    let catalog = blob(root.path(), include_bytes!("../../../catalog.json"));
    let marker = blob(root.path(), b"untrusted code must never run");
    let plugins = tree(root.path(), &format!("{mode} blob {record}\t{name}\n"));
    let registry = tree(
        root.path(),
        &format!("100644 blob {metadata}\tmetadata.json\n040000 tree {plugins}\tplugins\n"),
    );
    let revision = tree(
        root.path(),
        &format!(
            "100644 blob {marker}\tbuild.rs\n100644 blob {catalog}\tcatalog.json\n040000 tree {registry}\tregistry\n"
        ),
    );
    (root, revision)
}

#[test]
fn local_git_reads_only_blobs_without_a_checkout_or_commit() {
    let (repo, revision) = fixture("100644", "example.notes.json");
    assert!(!repo.path().join("build.rs").exists());
    let source = read_source_git(repo.path(), &revision).unwrap();
    assert_eq!(source.records[0].id.as_str(), "example.notes");
    assert!(!repo.path().join("build.rs").exists());
    assert!(!repo.path().join("registry").exists());
}

#[test]
fn invalid_revisions_modes_and_record_names_fail() {
    let (repo, revision) = fixture("100644", "example.notes.json");
    for rev in ["HEAD", "--help", "main:registry", "0123", &"a".repeat(41)] {
        assert!(read_source_git(repo.path(), rev).is_err());
    }
    read_source_git(repo.path(), &revision).unwrap();
    for (mode, name) in [
        ("100755", "example.notes.json"),
        ("120000", "example.notes.json"),
        ("100644", "other.notes.json"),
        ("100644", "build.rs"),
    ] {
        let (repo, revision) = fixture(mode, name);
        assert!(read_source_git(repo.path(), &revision).is_err());
    }
}

fn local_source() -> TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("registry/plugins")).unwrap();
    fs::write(
        root.path().join("registry/metadata.json"),
        common::metadata(),
    )
    .unwrap();
    fs::write(
        root.path().join("registry/plugins/example.notes.json"),
        serde_json::to_vec(&common::record()).unwrap(),
    )
    .unwrap();
    root
}

#[test]
fn local_directory_reads_the_same_allowlisted_source() {
    let root = local_source();
    assert_eq!(read_source_directory(root.path()).unwrap().records.len(), 1);
    fs::write(
        root.path().join("registry/plugins/unknown.txt"),
        b"unexpected",
    )
    .unwrap();
    assert!(read_source_directory(root.path()).is_err());
}

#[cfg(unix)]
#[test]
fn local_directory_rejects_symlinks_and_executable_json() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = local_source();
    let record = root.path().join("registry/plugins/example.notes.json");
    fs::set_permissions(&record, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(read_source_directory(root.path()).is_err());
    fs::remove_file(&record).unwrap();
    symlink(root.path().join("registry/metadata.json"), &record).unwrap();
    assert!(read_source_directory(root.path()).is_err());
}
