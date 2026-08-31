mod common;

use chronlyt_plugin_contracts::PluginCatalogV1;
use chronlyt_registry::{RegistrySourceV1, generate_catalog};

#[test]
fn repeated_generation_is_byte_identical_and_derives_integrity() {
    let source = common::source();
    let mut fetcher = common::MemoryFetcher::valid();
    let first = generate_catalog(&source, &common::previous_catalog(), &mut fetcher).unwrap();
    assert_eq!(fetcher.calls, 1);
    let second = generate_catalog(
        &source,
        &common::previous_catalog(),
        &mut common::MemoryFetcher::valid(),
    )
    .unwrap();
    assert_eq!(first, second);
    assert!(first.ends_with(b"\n"));
    assert!(!first.ends_with(b"\n\n"));
    let catalog = PluginCatalogV1::parse(&first).unwrap();
    let entry = &catalog.plugins[0];
    assert!(!entry.official && !entry.verified);
    assert_eq!(entry.downloads, None);
    assert_eq!(entry.stars, None);
    assert_eq!(entry.artifact.size, fetcher.bytes.len() as u64);
    use sha2::{Digest, Sha256};
    assert_eq!(
        entry.artifact.sha256,
        format!("{:x}", Sha256::digest(&fetcher.bytes))
    );
    assert_eq!(entry.description, "A bounded notes page.");
}

#[test]
fn empty_generation_matches_bootstrap_bytes_without_network() {
    let source = RegistrySourceV1::from_blobs(&common::metadata(), &[]).unwrap();
    let mut fetcher = common::MemoryFetcher::valid();
    let result = generate_catalog(&source, &common::previous_catalog(), &mut fetcher).unwrap();
    assert_eq!(result, include_bytes!("../../../catalog.json"));
    assert_eq!(fetcher.calls, 0);
}

#[test]
fn same_version_reupload_and_rewritten_created_timestamp_are_rejected() {
    let mut source = common::source();
    let previous = PluginCatalogV1::parse(
        &generate_catalog(
            &source,
            &common::previous_catalog(),
            &mut common::MemoryFetcher::valid(),
        )
        .unwrap(),
    )
    .unwrap();
    let mut replacement = common::MemoryFetcher {
        bytes: common::package("example.notes", "1.0.0", "Changed bytes"),
        calls: 0,
    };
    assert!(generate_catalog(&source, &previous, &mut replacement).is_err());
    source.records[0].created_at = "2026-08-28T00:00:00Z".into();
    assert!(generate_catalog(&source, &previous, &mut common::MemoryFetcher::valid()).is_err());
}

#[test]
fn metadata_and_url_changes_work_but_version_changes_must_advance() {
    let mut source = common::source();
    let previous = PluginCatalogV1::parse(
        &generate_catalog(
            &source,
            &common::previous_catalog(),
            &mut common::MemoryFetcher::valid(),
        )
        .unwrap(),
    )
    .unwrap();
    source.records[0].summary = "Updated description".into();
    source.records[0].release.artifact_url =
        "https://github.com/example/notes/releases/download/v1.0.0/copy.chronlyt-plugin".into();
    generate_catalog(&source, &previous, &mut common::MemoryFetcher::valid()).unwrap();
    for (version, accepted) in [("0.9.0", false), ("1.0.0+build.1", false), ("1.0.1", true)] {
        source.records[0].release.version = version.into();
        let mut fetcher = common::MemoryFetcher {
            bytes: common::package("example.notes", version, "A bounded notes page."),
            calls: 0,
        };
        assert_eq!(
            generate_catalog(&source, &previous, &mut fetcher).is_ok(),
            accepted,
            "{version}"
        );
    }
}

#[test]
fn mismatched_package_identity_and_invalid_components_fail_closed() {
    for bytes in [
        common::package("example.other", "1.0.0", "test"),
        common::package("example.notes", "2.0.0", "test"),
        b"not zip".to_vec(),
    ] {
        let mut fetcher = common::MemoryFetcher { bytes, calls: 0 };
        assert!(
            generate_catalog(&common::source(), &common::previous_catalog(), &mut fetcher).is_err()
        );
    }
}

#[test]
fn catalog_limit_stops_accumulating_entries_before_fetching_every_artifact() {
    use chronlyt_registry::{ArtifactFetcher, FetchBudget, RegistryResult};
    struct LargeEntries(usize);
    impl ArtifactFetcher for LargeEntries {
        fn fetch(&mut self, _: &str, _: FetchBudget) -> RegistryResult<Vec<u8>> {
            let id = format!("example.n{:02}", self.0);
            self.0 += 1;
            Ok(common::package_with_compatibility(
                &id,
                "1.0.0",
                "test",
                &format!("{}>=0.2.0", " ".repeat(100_000)),
            ))
        }
    }
    let blobs: Vec<_> = (0..40)
        .map(|n| {
            let id = format!("example.n{n:02}");
            let mut record = common::record();
            record["id"] = id.clone().into();
            (format!("{id}.json"), serde_json::to_vec(&record).unwrap())
        })
        .collect();
    let source = RegistrySourceV1::from_blobs(&common::metadata(), &blobs).unwrap();
    let mut fetcher = LargeEntries(0);
    let error = generate_catalog(&source, &common::previous_catalog(), &mut fetcher).unwrap_err();
    assert!(matches!(
        error,
        chronlyt_registry::RegistryError::Limit("catalog bytes")
    ));
    assert!(
        fetcher.0 < 25,
        "fetched {} entries before rejecting",
        fetcher.0
    );
}

#[test]
fn record_order_does_not_change_bytes_and_failed_fetch_does_not_skip_entries() {
    use chronlyt_registry::{ArtifactFetcher, FetchBudget, RegistryError, RegistryResult};
    struct TwoReleases {
        calls: usize,
        fail_second: bool,
        last_budget: Option<FetchBudget>,
    }
    impl ArtifactFetcher for TwoReleases {
        fn fetch(&mut self, _: &str, budget: FetchBudget) -> RegistryResult<Vec<u8>> {
            if let Some(prior) = self.last_budget {
                assert!(budget.max_bytes < prior.max_bytes);
                assert_eq!(budget.deadline, prior.deadline);
            }
            self.last_budget = Some(budget);
            self.calls += 1;
            if self.calls == 2 && self.fail_second {
                return Err(RegistryError::Transport);
            }
            Ok(common::package(
                if self.calls == 1 {
                    "example.alpha"
                } else {
                    "example.notes"
                },
                "1.0.0",
                "test",
            ))
        }
    }
    let mut source = common::source();
    let mut first = source.records[0].clone();
    first.id = chronlyt_plugin_contracts::PluginId::parse("example.alpha").unwrap();
    source.records.push(first);
    let mut fetcher = TwoReleases {
        calls: 0,
        fail_second: false,
        last_budget: None,
    };
    let result = generate_catalog(&source, &common::previous_catalog(), &mut fetcher).unwrap();
    source.records.reverse();
    let mut fetcher = TwoReleases {
        calls: 0,
        fail_second: false,
        last_budget: None,
    };
    assert_eq!(
        generate_catalog(&source, &common::previous_catalog(), &mut fetcher).unwrap(),
        result
    );
    let mut fetcher = TwoReleases {
        calls: 0,
        fail_second: true,
        last_budget: None,
    };
    assert!(matches!(
        generate_catalog(&source, &common::previous_catalog(), &mut fetcher),
        Err(RegistryError::Transport)
    ));
}
