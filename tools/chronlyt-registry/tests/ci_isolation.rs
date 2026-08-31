mod common;
#[path = "common/pr_fixture.rs"]
mod pr_fixture;
use std::{collections::BTreeSet, fs, path::Path};
use yaml_rust2::{Yaml, YamlLoader};

const CHECKOUT: &str = "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1";
fn workflow(name: &str) -> (String, Yaml) {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.github/workflows")
        .join(name);
    let text = fs::read_to_string(file).unwrap();
    assert!(text.len() < 16 * 1024);
    let mut docs = YamlLoader::load_from_str(&text).unwrap();
    assert_eq!(docs.len(), 1);
    (text, docs.remove(0))
}
fn keys(value: &Yaml) -> BTreeSet<&str> {
    value
        .as_hash()
        .unwrap()
        .keys()
        .map(|key| key.as_str().unwrap())
        .collect()
}
fn only(value: &Yaml, allowed: &[&str]) {
    assert!(
        keys(value).iter().all(|key| allowed.contains(key)),
        "unexpected YAML key"
    );
}
fn readonly(value: &Yaml, pr: bool) {
    assert_eq!(
        keys(value),
        if pr {
            BTreeSet::from(["contents", "pull-requests"])
        } else {
            BTreeSet::from(["contents"])
        }
    );
    assert!(
        value
            .as_hash()
            .unwrap()
            .values()
            .all(|v| v.as_str() == Some("read"))
    );
}

#[test]
fn target_workflow_and_validator_are_the_same_trusted_revision() {
    let (text, workflow) = workflow("registry-check.yml");
    only(&workflow, &["name", "on", "permissions", "jobs"]);
    assert_eq!(
        keys(&workflow["on"]),
        BTreeSet::from(["pull_request_target"])
    );
    let trigger = &workflow["on"]["pull_request_target"];
    assert_eq!(keys(trigger), BTreeSet::from(["branches", "types"]));
    assert_eq!(
        trigger["branches"].as_vec().unwrap(),
        &[Yaml::String("main".into())]
    );
    readonly(&workflow["permissions"], true);
    assert_eq!(
        keys(&workflow["jobs"]),
        BTreeSet::from(["registry-data-validation"])
    );
    let job = &workflow["jobs"]["registry-data-validation"];
    only(
        job,
        &[
            "runs-on",
            "timeout-minutes",
            "permissions",
            "defaults",
            "env",
            "steps",
        ],
    );
    readonly(&job["permissions"], true);
    assert_eq!(job["runs-on"].as_str(), Some("ubuntu-24.04"));
    assert_eq!(job["timeout-minutes"].as_i64(), Some(15));
    assert_eq!(
        job["defaults"]["run"]["working-directory"].as_str(),
        Some("trusted")
    );
    assert_eq!(
        job["env"]["TRUSTED_SHA"].as_str(),
        Some("${{ github.workflow_sha }}")
    );
    assert_eq!(
        job["env"]["PR_NUMBER"].as_str(),
        Some("${{ github.event.pull_request.number }}")
    );
    assert_eq!(
        job["env"]["PR_HEAD_SHA"].as_str(),
        Some("${{ github.event.pull_request.head.sha }}")
    );
    assert_eq!(
        keys(&job["env"]),
        BTreeSet::from(["TRUSTED_SHA", "PR_NUMBER", "PR_HEAD_SHA", "TARGET_BRANCH"])
    );
    let steps = job["steps"].as_vec().unwrap();
    assert_eq!(steps.len(), 4);
    let checkout = &steps[1];
    assert_eq!(checkout["uses"].as_str(), Some(CHECKOUT));
    only(checkout, &["name", "uses", "with"]);
    assert_eq!(
        checkout["with"]["repository"].as_str(),
        Some("n3d7/Chronlyt-Plugins")
    );
    assert_eq!(
        checkout["with"]["ref"].as_str(),
        Some("${{ github.workflow_sha }}")
    );
    assert_eq!(checkout["with"]["path"].as_str(), Some("trusted"));
    for key in ["persist-credentials", "lfs", "submodules", "clean"] {
        assert_eq!(checkout["with"][key].as_bool(), Some(false), "{key}");
    }
    only(
        &checkout["with"],
        &[
            "repository",
            "ref",
            "path",
            "persist-credentials",
            "lfs",
            "submodules",
            "clean",
            "fetch-depth",
            "set-safe-directory",
        ],
    );
    for (i, step) in steps.iter().enumerate().filter(|(i, _)| *i != 1) {
        only(step, &["name", "run", "shell", "working-directory", "env"]);
        assert_eq!(step["shell"].as_str(), Some("bash"));
        let body = step["run"].as_str().unwrap();
        assert!(
            !body.contains("${{"),
            "expressions must enter as env, not shell source"
        );
        for (name, value) in [
            ("CARGO_HOME", "${{ runner.temp }}/registry-cargo"),
            ("CARGO_TARGET_DIR", "${{ runner.temp }}/registry-target"),
            ("RUSTUP_HOME", "${{ runner.temp }}/registry-rustup"),
        ] {
            assert_eq!(step["env"][name].as_str(), Some(value));
        }
        if i != 3 {
            assert_eq!(
                keys(&step["env"]),
                BTreeSet::from(["CARGO_HOME", "CARGO_TARGET_DIR", "RUSTUP_HOME"])
            );
        }
    }
    let guard = steps[0]["run"].as_str().unwrap();
    for expected in [
        "GITHUB_REPOSITORY",
        "GITHUB_EVENT_NAME",
        "GITHUB_WORKFLOW_REF",
        "TARGET_BRANCH",
        "TRUSTED_SHA",
        "PR_NUMBER",
        "PR_HEAD_SHA",
        ".github/workflows/registry-check.yml@refs/heads/main",
    ] {
        assert!(guard.contains(expected));
    }
    let build = steps[2]["run"].as_str().unwrap();
    assert!(build.contains("git rev-parse HEAD"));
    assert!(build.contains("cargo build --locked -p chronlyt-registry"));
    assert!(!build.contains("PR_HEAD_SHA"));
    let check = &steps[3];
    assert_eq!(
        keys(&check["env"]),
        BTreeSet::from([
            "CHRONLYT_REGISTRY_GITHUB_TOKEN",
            "CARGO_HOME",
            "CARGO_TARGET_DIR",
            "RUSTUP_HOME"
        ])
    );
    assert_eq!(
        check["env"]["CHRONLYT_REGISTRY_GITHUB_TOKEN"].as_str(),
        Some("${{ github.token }}")
    );
    let body = check["run"].as_str().unwrap();
    assert!(body.contains("--source-pr \"$PR_NUMBER\" --source-rev \"$PR_HEAD_SHA\""));
    assert!(body.contains("--base-rev \"$TRUSTED_SHA\""));
    assert!(!body.contains("cargo "));
    for forbidden in [
        "secrets.",
        "secrets[",
        "secrets:",
        "environment:",
        "id-token:",
        ": write",
        "continue-on-error",
        "actions/cache",
        "download-artifact",
        "upload-artifact",
        "workflow_call",
        "allow-unsafe-pr-checkout",
        "git fetch",
        "--source-git",
        "head.repo",
        "head.ref",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}");
    }
}

#[test]
fn candidate_code_runs_only_in_an_unprivileged_separate_workflow() {
    let (text, workflow) = workflow("contracts-and-canary.yml");
    assert!(workflow["on"]["pull_request"].as_hash().is_some());
    assert!(workflow["on"]["pull_request_target"].is_badvalue());
    readonly(&workflow["permissions"], false);
    for (_, job) in workflow["jobs"].as_hash().unwrap() {
        readonly(&job["permissions"], false);
        assert_eq!(job["runs-on"].as_str(), Some("ubuntu-24.04"));
    }
    for forbidden in [
        "secrets.",
        "secrets[",
        "secrets:",
        "environment:",
        "id-token:",
        ": write",
        "CHRONLYT_REGISTRY_GITHUB_TOKEN",
        "actions/cache",
        "download-artifact",
        "upload-artifact",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}");
    }
}

#[test]
fn contributor_workflows_build_scripts_and_forged_outputs_are_never_read_or_run() {
    use chronlyt_registry::{generate_catalog, git_data::PrDataReader};
    use pr_fixture::{entry, mock, sha};
    use serde_json::json;
    let marker_root = tempfile::tempdir().unwrap();
    let marker = marker_root.path().join("executed");
    let malicious = format!("touch {}", marker.display());
    let catalog = generate_catalog(
        &common::source(),
        &common::previous_catalog(),
        &mut common::MemoryFetcher::valid(),
    )
    .unwrap();
    let mut mock = mock();
    mock.blob('2', &catalog);
    mock.blob('8', malicious.as_bytes());
    mock.tree(
        '7',
        json!([entry(
            "config.toml",
            '8',
            "blob",
            "100644",
            Some(malicious.len())
        )]),
    );
    mock.tree(
        '6',
        json!([entry(
            "registry-check.yml",
            '8',
            "blob",
            "100644",
            Some(malicious.len())
        )]),
    );
    mock.tree(
        '5',
        json!([entry("workflows", '6', "tree", "040000", None)]),
    );
    mock.tree(
        'c',
        json!([
            entry("registry", 'd', "tree", "040000", None),
            entry("catalog.json", '2', "blob", "100644", Some(catalog.len())),
            entry(".github", '5', "tree", "040000", None),
            entry(".cargo", '7', "tree", "040000", None),
            entry("build.rs", '8', "blob", "100755", Some(malicious.len())),
            entry(
                "validator-artifact",
                '8',
                "blob",
                "100755",
                Some(malicious.len())
            ),
            entry("cache", '8', "blob", "100644", Some(malicious.len()))
        ]),
    );
    // Any attempted read outside the data allowlist panics in Mock::get.
    for (kind, id) in [
        ("trees", '5'),
        ("trees", '6'),
        ("trees", '7'),
        ("blobs", '8'),
    ] {
        mock.bodies
            .remove(&format!("{}git/{kind}/{}", pr_fixture::API, sha(id)));
    }
    let mut reader = PrDataReader::with_transport(mock);
    let data = reader.read(7, &sha('a')).unwrap();
    reader.verify_freshness(&data, &sha('b')).unwrap();
    let generated = generate_catalog(
        &data.source,
        &common::previous_catalog(),
        &mut common::MemoryFetcher::valid(),
    )
    .unwrap();
    assert_eq!(data.catalog_bytes, generated);
    reader.verify_freshness(&data, &sha('b')).unwrap();
    assert!(!marker.exists());
}

#[test]
fn api_failure_never_falls_back_to_contributor_checkout_or_artifacts() {
    struct Offline;
    impl chronlyt_registry::transport::HttpTransport for Offline {
        fn get(
            &mut self,
            _: &str,
            _: std::time::Duration,
        ) -> chronlyt_registry::RegistryResult<chronlyt_registry::transport::HttpResponse> {
            Err(chronlyt_registry::RegistryError::Transport)
        }
    }
    assert!(
        chronlyt_registry::git_data::PrDataReader::with_transport(Offline)
            .read(7, &pr_fixture::sha('a'))
            .is_err()
    );
}
