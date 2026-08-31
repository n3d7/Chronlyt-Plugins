# Registry source and generated catalog

Community intake is closed during bootstrap. The root `catalog.json` is the
existing Chronlyt v1 endpoint. It is generated **and committed in the same PR**
as registry changes. No post-merge writer, catalog signatures or trust service
exists. Passing registry checks never replaces the host's independent package,
runtime and permission checks.

## Source

`registry/metadata.json` contains `schema_version: 1` and an explicit RFC3339
`generated_at`. Do not substitute a wall-clock build timestamp.
`registry/plugins/<plugin-id>.json` contains exactly:

```json
{
  "schema_version": 1,
  "id": "example.notes",
  "release": {
    "version": "1.0.0",
    "artifact_url": "https://github.com/example/notes/releases/download/v1.0.0/notes.chronlyt-plugin"
  },
  "repository_url": "https://github.com/example/notes",
  "summary": "Private notes",
  "publisher": "Example",
  "license": "MIT",
  "categories": ["writing"],
  "tags": ["notes"],
  "created_at": "2026-08-29T09:00:00Z",
  "updated_at": "2026-08-29T10:00:00Z",
  "changelog": "Initial release."
}
```

This is illustrative, not a published plugin. Author source lives in its own
repository; release artifacts use `.chronlyt-plugin`. Records cannot supply
size/digest/trust flags. The filename must match the manifest identity. Unknown
fields, executable/symlink/submodule modes and unknown paths in `registry` fail.
Only an empty `.gitkeep` is allowed alongside record files.

The generator fetches each artifact once, bounds it, verifies the archive,
internal hashes and static canonical WIT contract, then derives metadata and
SHA-256/size from those same bytes. It never instantiates a guest. Entries are
sorted by ID, meaningful input arrays retain order, and typed pretty JSON has
exactly one final LF. All entries default to `official=false`, `verified=false`,
with no live download/star statistics.

For an unchanged ID/version, digest and size must equal the trusted base catalog,
even when the release URL changes. Publish changed bytes as a forward SemVer
release; a build-metadata-only change is not forward precedence. Existing
`created_at` cannot be rewritten; require `created_at <= updated_at <= generated_at`.
Missing artifacts or any invalid entry fail the whole operation. Never skip a
failed record or silently refresh its integrity values.

## Commands

From this repository, use the pinned Rust toolchain and lockfile. `BASE_SHA` and
`HEAD_SHA` below must be actual full 40-character lowercase Git revisions; no
branch names, revision expressions or invented pins. `BASE_REPO` is a trusted
local checkout containing the baseline objects, not a PR-controlled checkout.

```sh
cargo run --locked -p chronlyt-registry -- generate \
  --source-dir "$SOURCE_REPO" --base-git "$BASE_REPO" --base-rev "$BASE_SHA"
cargo run --locked -p chronlyt-registry -- check \
  --source-git "$SOURCE_REPO" --source-rev "$HEAD_SHA" \
  --base-git "$BASE_REPO" --base-rev "$BASE_SHA"
cargo run --locked -p chronlyt-registry -- schemas --write
cargo run --locked -p chronlyt-registry -- schemas --check
```

`generate` writes only the explicit local source root's catalog, using a synced
temporary file and rename after full success. A failed generation preserves the
old output; output symlinks are rejected. Do not mutate the local source directory
concurrently: this author mode is not the hostile PR filesystem boundary.
`check` is read-only and requires an exact byte match, including whitespace/LF.
For initial bootstrap only, an absent base catalog permits an **empty** registry;
a nonempty source, invalid base catalog or unsafe base blob fails closed.

The trusted CI binary uses a separate mode:

```sh
"$CARGO_TARGET_DIR/debug/chronlyt-registry" check \
  --source-pr "$PR_NUMBER" --source-rev "$HEAD_SHA" \
  --base-git "$BASE_REPO" --base-rev "$TRUSTED_SHA"
```

Build that binary from trusted base code, never PR-head Cargo/build scripts.
The dedicated optional `CHRONLYT_REGISTRY_GITHUB_TOKEN` is only for this step's
fixed repository API calls. No ambient GH_TOKEN/GITHUB_TOKEN/proxy credentials
are read. The API client has redirects disabled; the separate credential-free
artifact client validates the initial URL and every allowed redirect. PR mode
checks current open PR/head/base-main identity before acquisition and again
after successful generation; stale trusted-main or PR-head revisions fail.
PR filenames, URLs, workflows and executable artifacts are never instructions.

## Bounds and schemas

Wire/package limits come from `chronlyt-plugin-contracts`; see generated
`contracts/generated/v1/limits.json`. Registry-only intake policy is 4 KiB
metadata, 64 KiB per source record, 8 MiB decoded source, 16 MiB encoded GitHub
API responses, 1 MiB per tree, 2,100 API requests and 10 minutes acquisition.
Artifact transfer is sequential: at most three redirects, 5 seconds connect,
30 seconds total per artifact, 256 MiB aggregate and 10 minutes generation.
HTTP bodies have streaming caps; advertised lengths must match actual lengths.
No fallback to clone URLs, repository tarballs, PR checkout or arbitrary fetch.

Generated Draft 2020-12 schemas derive Serde shapes from the optional `schema`
feature. `registry-source.schema.json` describes a record; its
`$defs/RegistryMetadataV1` describes metadata. `host-payloads.schema.json` bundles
the existing named guest DTOs; choose the appropriate `$defs` for the WIT call.
Missing/null/default and unknown-field behavior follow the deserialization
contract, including the nullable Timeline patch and tolerant output DTOs.

Schemas are documentation/editor assistance, **not standalone security
validation**. `x-chronlyt-max-utf8-bytes` / `x-chronlyt-min-utf8-bytes` are UTF-8
byte limits (on root objects, serialized input bytes), not `maxLength`.
`x-chronlyt-trimmed-max-codepoints` applies after Rust trimming. UI depth/node/input
extensions describe aggregate tree budgets. Standard JSON Schema validators may
ignore these extensions. Always run shared Rust validation for bytes, IDs,
SemVer, timestamps, URL/path rules, duplicate IDs, internal hashes and WIT.
Generated `capabilities.json` comes from the canonical Serde enum; limit values
are enumerated from the same declarations used by Rust validation.

## Bootstrap and external readiness

Local implementation/test success does not mean the registry is open. Before
accepting community entries, an explicitly authorized maintainer must:

- Review and publish the bootstrap workflow, validator, contracts, lockfile and
  empty catalog to protected default `main`. While they exist only in a PR there
  is no trusted target result; never substitute candidate workflow/tooling or
  treat a missing/skipped target job as success.
- Read back effective repository/ruleset configuration: PR-only main, required
  code-owner approval, dismissal of stale approvals, required relevant checks,
  and no routine direct/force-push bypass. CODEOWNERS alone grants none of this.
- Authorize an isolated fork test. Change target YAML, `.cargo/config.toml`,
  build.rs and executable/cache/artifact markers in its PR. Verify only bounded
  data is acquired by the main-owned target workflow, with no marker execution.
  Test stale heads/base revisions and a candidate job forging the same green
  check name. A matching check name/GitHub Actions app is not workflow identity.
- Verify the **actual** merge gate associates the authentic target run with the
  current PR/head/base tuple. Record workflow ID/path/event, run ID/attempt,
  trusted main SHA, PR number/head SHA and computed catalog SHA-256. Logs contain
  `registry-run` and, only after successful freshness checking, `registry-proof`.
  Those strings are not signatures: authenticate their run metadata through
  GitHub, not screenshots or candidate-supplied text/artifacts.
- If workflow-level enforcement is unavailable, keep auto-merge and automated
  community intake closed. Mandatory maintainer/code-owner review must verify
  the authentic target run for the exact current head/base before any merge.
  Do not work around this with status-write privileges or a privileged publisher.

The target job uses a single fixed-repository checkout at `github.workflow_sha`,
fresh Cargo/target/rustup directories and a Cargo build with cleared environment.
Only the built validator's step gets a read-only API token. `runner.temp` paths
belong in **step env**, not job env, as confirmed by actionlint and GitHub's
[context availability](https://docs.github.com/en/actions/reference/workflows-and-actions/contexts#context-availability).
No workflow uses candidate caches/artifacts, secrets, OIDC or write permissions.

Local isolation tests parse the YAML structure and exercise hostile PR data with
a valid registry record, not merely a rejection path. They are regression tests,
not a universal workflow-policy engine or proof of GitHub configuration. The
reviewed actionlint binary checks workflow grammar/expressions; when shellcheck
and pyflakes are absent their optional integrations are disabled, not claimed.
