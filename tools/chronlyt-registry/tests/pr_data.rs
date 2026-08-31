mod common;
#[path = "common/pr_fixture.rs"]
mod pr_fixture;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chronlyt_registry::git_data::PrDataReader;
use pr_fixture::{API, mock, sha};
use serde_json::json;

#[test]
fn pr_is_bounded_data_not_contributor_urls_or_executables() {
    let mut reader = PrDataReader::with_transport(mock());
    let snapshot = reader.read(7, &sha('a')).unwrap();
    assert_eq!(snapshot.head_sha, sha('a'));
    assert_eq!(snapshot.source.records.len(), 1);
    reader.verify_freshness(&snapshot, &sha('b')).unwrap();
}

#[test]
fn wrong_identity_target_base_or_truncated_tree_fail() {
    for field in [
        "number",
        "head",
        "target",
        "repository",
        "base",
        "truncated",
    ] {
        let mut mock = mock();
        match field {
            "number" => mock.change("pulls/7", |v| v["number"] = json!(8)),
            "head" => mock.change("pulls/7", |v| v["head"]["sha"] = json!(sha('9'))),
            "target" => mock.change("pulls/7", |v| v["base"]["ref"] = json!("other")),
            "repository" => mock.change("pulls/7", |v| {
                v["base"]["repo"]["full_name"] = json!("evil/repo")
            }),
            "base" => mock.change("git/ref/heads/main", |v| {
                v["object"]["sha"] = json!(sha('9'))
            }),
            _ => mock.change(&format!("git/trees/{}", sha('c')), |v| {
                v["truncated"] = json!(true)
            }),
        }
        assert!(
            PrDataReader::with_transport(mock)
                .read(7, &sha('a'))
                .is_err(),
            "{field}"
        );
    }
}

#[test]
fn invalid_blob_modes_sizes_and_encoding_fail() {
    for mode in ["100755", "120000", "160000"] {
        let mut mock = mock();
        mock.change(&format!("git/trees/{}", sha('e')), |v| {
            v["tree"][0]["mode"] = json!(mode)
        });
        assert!(
            PrDataReader::with_transport(mock)
                .read(7, &sha('a'))
                .is_err()
        );
    }
    for (field, value) in [
        ("size", json!(999999)),
        ("encoding", json!("utf-8")),
        ("content", json!("!bad!")),
        ("sha", json!(sha('9'))),
    ] {
        let mut mock = mock();
        mock.change(&format!("git/blobs/{}", sha('1')), |v| v[field] = value);
        assert!(
            PrDataReader::with_transport(mock)
                .read(7, &sha('a'))
                .is_err()
        );
    }
}

#[test]
fn stale_result_cannot_pass_freshness_or_trusted_revision_check() {
    let mut reader = PrDataReader::with_transport(mock());
    let snapshot = reader.read(7, &sha('a')).unwrap();
    assert!(reader.verify_freshness(&snapshot, &sha('9')).is_err());
    let mut changed = mock();
    changed.change("pulls/7", |v| v["head"]["sha"] = json!(sha('9')));
    assert!(
        PrDataReader::with_transport(changed)
            .verify_freshness(&snapshot, &sha('b'))
            .is_err()
    );
}

#[test]
fn oversized_or_unknown_paths_fail_before_blob_fetch_or_decode() {
    for oversized in [false, true] {
        let mut mock = mock();
        mock.change(&format!("git/trees/{}", sha('e')), |v| {
            if oversized {
                v["tree"][0]["size"] = json!(65537);
            } else {
                v["tree"][0]["path"] = json!("build.rs");
            }
        });
        mock.bodies.remove(&format!("{API}git/blobs/{}", sha('1')));
        assert!(
            PrDataReader::with_transport(mock)
                .read(7, &sha('a'))
                .is_err()
        );
    }
    let mut mock = mock();
    mock.bodies.insert(
        format!("{API}git/trees/{}", sha('c')),
        vec![b' '; 1024 * 1024 + 1],
    );
    assert!(
        PrDataReader::with_transport(mock)
            .read(7, &sha('a'))
            .is_err()
    );
}

#[test]
fn decoded_content_must_match_the_advertised_blob_size() {
    let mut mock = mock();
    mock.change(&format!("git/blobs/{}", sha('1')), |v| {
        let size = v["size"].as_u64().unwrap() as usize;
        v["content"] = json!(STANDARD.encode(vec![b'a'; size + 1]));
    });
    assert!(
        PrDataReader::with_transport(mock)
            .read(7, &sha('a'))
            .is_err()
    );
}
