mod common;
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_chronlyt-registry"))
        .args(args)
        .env_remove("CHRONLYT_REGISTRY_GITHUB_TOKEN")
        .output()
        .unwrap()
}
fn git(repo: &Path, args: &[&str], input: &[u8]) -> String {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(result.status.success());
    String::from_utf8(result.stdout).unwrap().trim().into()
}
fn base(catalog: Option<&[u8]>) -> (tempfile::TempDir, String) {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "--bare"], b"");
    let entries = catalog
        .map(|bytes| {
            let sha = git(root.path(), &["hash-object", "-w", "--stdin"], bytes);
            format!("100644 blob {sha}\tcatalog.json\n")
        })
        .unwrap_or_default();
    let rev = git(root.path(), &["mktree"], entries.as_bytes());
    (root, rev)
}
fn source() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("registry/plugins")).unwrap();
    fs::write(
        root.path().join("registry/metadata.json"),
        common::metadata(),
    )
    .unwrap();
    root
}

#[test]
fn generation_bootstraps_empty_only_and_preserves_output_on_failure() {
    let (repo, rev) = base(None);
    let source = source();
    let args = [
        "generate",
        "--source-dir",
        source.path().to_str().unwrap(),
        "--base-git",
        repo.path().to_str().unwrap(),
        "--base-rev",
        &rev,
    ];
    let result = run(&args);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let output = source.path().join("catalog.json");
    let old = fs::read(&output).unwrap();
    assert_eq!(old, include_bytes!("../../../catalog.json"));
    fs::write(
        source.path().join("registry/plugins/example.notes.json"),
        serde_json::to_vec(&common::record()).unwrap(),
    )
    .unwrap();
    assert!(!run(&args).status.success());
    assert_eq!(fs::read(&output).unwrap(), old);
    assert_eq!(fs::read_dir(source.path()).unwrap().count(), 2);
}

#[test]
fn corrupt_base_cannot_be_treated_as_empty_and_invalid_options_fail() {
    let (repo, rev) = base(Some(b"broken"));
    let source = source();
    assert!(
        !run(&[
            "generate",
            "--source-dir",
            source.path().to_str().unwrap(),
            "--base-git",
            repo.path().to_str().unwrap(),
            "--base-rev",
            &rev
        ])
        .status
        .success()
    );
    assert!(!source.path().join("catalog.json").exists());
    for args in [
        vec!["check", "--source-pr", "0"],
        vec!["check", "--source-pr", "1", "--source-git", "."],
        vec!["schemas", "--check", "--write"],
        vec!["generate", "--base-rev", "HEAD"],
        vec!["pack"],
    ] {
        assert!(!run(&args).status.success(), "{args:?}");
    }
}

#[cfg(unix)]
#[test]
fn generation_never_follows_an_output_symlink() {
    let (repo, rev) = base(Some(include_bytes!("../../../catalog.json")));
    let source = source();
    let target = source.path().join("registry/metadata.json");
    std::os::unix::fs::symlink(&target, source.path().join("catalog.json")).unwrap();
    assert!(
        !run(&[
            "generate",
            "--source-dir",
            source.path().to_str().unwrap(),
            "--base-git",
            repo.path().to_str().unwrap(),
            "--base-rev",
            &rev
        ])
        .status
        .success()
    );
    assert_eq!(fs::read(&target).unwrap(), common::metadata());
}

#[test]
fn git_check_requires_exact_committed_bytes_and_ignores_timezone() {
    let (repo, base_rev) = base(Some(include_bytes!("../../../catalog.json")));
    let metadata = git(
        repo.path(),
        &["hash-object", "-w", "--stdin"],
        &common::metadata(),
    );
    let empty = git(repo.path(), &["mktree"], b"");
    let registry = git(
        repo.path(),
        &["mktree"],
        format!("100644 blob {metadata}\tmetadata.json\n040000 tree {empty}\tplugins\n").as_bytes(),
    );
    for exact in [true, false] {
        let mut bytes = include_bytes!("../../../catalog.json").to_vec();
        if !exact {
            bytes.push(b'\n');
        }
        let catalog = git(repo.path(), &["hash-object", "-w", "--stdin"], &bytes);
        let source_rev = git(
            repo.path(),
            &["mktree"],
            format!("100644 blob {catalog}\tcatalog.json\n040000 tree {registry}\tregistry\n")
                .as_bytes(),
        );
        let args = [
            "check",
            "--source-git",
            repo.path().to_str().unwrap(),
            "--source-rev",
            &source_rev,
            "--base-git",
            repo.path().to_str().unwrap(),
            "--base-rev",
            &base_rev,
        ];
        for tz in ["UTC", "Pacific/Kiritimati"] {
            let result = Command::new(env!("CARGO_BIN_EXE_chronlyt-registry"))
                .args(args)
                .env("TZ", tz)
                .output()
                .unwrap();
            assert_eq!(result.status.success(), exact, "{tz}");
        }
    }
}
