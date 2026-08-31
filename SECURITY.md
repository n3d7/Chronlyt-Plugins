# Security boundary

Registry records, manifests, archives, assets, components and author repositories
are untrusted. Validation does not confer runtime authority, authenticity,
cryptographic catalog signing or a verified badge. Community intake is closed
until the documented [readiness gates](docs/registry.md#bootstrap-and-external-readiness)
have been demonstrated.

Report vulnerabilities privately using GitHub private vulnerability reporting
if enabled. Otherwise arrange a private reporting channel with the maintainer.
Do not post secrets, private source/data or weaponized artifacts in public issues.
No response-time guarantee or reporting address is invented by this policy.

## Invariants for changes and reviews

- The canonical WIT, Rust wire shapes and pure validators must remain compatible
  with the host's v1 API/package/data contracts. Shared host adoption must not
  leave a second manually maintained security validator. Engine versions, Store
  limits and host install/runtime policy are not public ABI versions.
- Enforce archive/file/manifest/component/UI/host-call/transfer bounds and hashes.
  Do not instantiate submitted guests to validate registry data. No shell input,
  native code execution, local file URL or arbitrary network fallback comes from
  registry fields. Errors must not echo signed URLs, raw input or credentials.
- Trusted `pull_request_target` orchestration and executable validator come from
  the same reviewed main revision, never the candidate workflow or checkout.
  Read PR data only through bounded fixed-repository GitHub blob endpoints.
  The API token is read-only, step-scoped and separate from artifact transport.
- Target workflows never reference repository/organization/environment secrets,
  OIDC/write permissions, private dependencies, candidate artifacts/caches or
  contributor-controlled scripts/actions/config. Ephemeral hosted runners and
  immutable reviewed action pins are mandatory. Token scopes alone do not
  disable the target event's secrets context: absence of secret injection is
  a separately reviewed property.
- Candidate code builds run only in separate unprivileged `pull_request` jobs.
  Their results or artifacts must never select the trusted validator. Required
  check names alone cannot authenticate a workflow or bind a fresh PR snapshot.
- Catalog generation is deterministic and must exactly match the same PR's
  committed root catalog. Reject changed bytes at the same version and require
  explicit host approval when a plugin update requests additional permissions.
- CODEOWNERS is review routing, not enforcement. Contracts/WIT, workflows,
  validator/generator, fixtures, dependency/toolchain files and any future trust
  metadata need enforced protected/code-owner review. No reviewer bypass or
  speculative trust automation is introduced by this milestone.

The host independently validates all downloaded packages and applies its own
stricter execution/install policy. Passing registry CI never weakens that layer.

## Known limitations

There is no catalog signature verification in the current client. SHA-256 pins
artifact bytes to catalog entries, not authors to a trusted identity. Signing
would require coordinated client support and separate key/release governance.
The ZIP library allocates metadata under a bounded compressed input before the
public entry-count guard; see [compatibility notes](docs/compatibility.md).
Local author mode assumes no concurrent filesystem mutation; hostile PR mode
does not read a contributor checkout. Static/WIT validation is not execution
testing. Actual GitHub enforcement/readiness is not established by local tests.
