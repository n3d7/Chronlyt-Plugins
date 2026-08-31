# Chronlyt v1 compatibility

## Package and component validation

`chronlyt-plugin-validator` exposes `inspect_package`, `validate_package` and
`validate_component_contract`. Archive validation returns immutable bytes, never
writes files, and does not instantiate a component. External catalog size/hash
are checked before ZIP inspection; internal component/asset hashes and declared
file sets are checked before returning any file list. Static component checking
is deliberately separate from archive inspection.

The static checker derives expected imports/exports from the canonical WIT.
It accepts missing unused host imports, subsets of the host's functions and extra
exports, as the current host does. It rejects foreign/WASI imports, wrong host
versions, missing exports and wrong function types. Parameter labels are not
part of the typed-call ABI. Nested module imports are resolved inside the
component and do not grant host capabilities. Guest start functions are never
run here. The host still applies its engine feature/resource policy at install.

Extraction found that zip 8 collapses duplicate central-directory names before
the original host's duplicate-name loop. The shared validator counts raw central
records at the library-resolved offset before trusting that deduplicated index.
This enforces the existing no-duplicates rule; coordinated host adoption must
reuse this check. Header parsing/ZIP64 resolution remains upstream zip's job;
the 16 MiB input cap applies before it is called and payload bounds precede
payload allocation/reads. No claim of a separate allocator budget inside the
upstream metadata parser is made.

The synthetic WAT corpus has matching public static/unchanged-host compile-only
outcomes, including a trapping start function which is not instantiated. This
is contract conformance evidence, not an execution or package provenance proof.

Public compatibility is defined by the versioned Chronlyt WIT/API and data and
package contracts, not by a Wasmtime or Rust toolchain version.

`chronlyt-plugin-contracts` package version 0.1.0 contains the unchanged v1 wire
contracts. The canonical WIT source is
`crates/chronlyt-plugin-contracts/wit/chronlyt-plugin.wit`, exported as
`chronlyt_plugin_contracts::WIT_SOURCE`. Its package is
`chronlyt:plugin@1.0.0`, world `chronlyt-plugin`.

## Extraction and adoption

This limited, authorized extraction originates from the Chronlyt-base Plugin
System v1 baseline `1847acb5e00e81676df4292e96b8ff364dbfee34`: manifest, catalog,
capability and declarative UI wire definitions, portable paths/URL rules and
guest-visible Timeline/storage fields. It is distributed under this repository's
MIT license, copyright n3d7. No private history, auth, database implementation,
runtime/lifecycle or user data is included.

Until coordinated adoption passes, the original host remains the compatibility
authority. The interim host/public copies are an extraction checkpoint, not two
independently maintained protocols. Adoption must replace matching private rules
with these imports and remove its source-maintained WIT copy.

Both public contracts and the future pure package validator will be consumed by
the host at the same real full Git revision with Cargo.lock. Crates.io publication
is deferred. There is no usable public revision containing these uncommitted
changes yet; local path integration does not constitute a published adoption.

## Preserved behavior

- Capabilities: storage.read, storage.write, timeline.read, timeline.write,
  notifications.show. No additional guest authority is introduced.
- Manifest/catalog/UI objects remain closed and versioned. Optional fields and
  defaults retain their original Serde behavior. Timeline/storage output DTOs
  that accepted unknown fields still do so.
- Timeline duration patches distinguish missing, null and an integer while
  decoding. Original serialization emits null for missing as well; callers must
  not assume patch serialization preserves that distinction.
- Most string limits are UTF-8 bytes. Timeline title/note bounds count Unicode
  scalar values, with the original title trimming and UTC timestamp normalization.
- Initial artifact URLs and redirect URLs have distinct policies. Query strings
  are permitted only on allowed object-storage redirects. URL parser normalization
  is preserved; these validators do not add a downloader or relax the host policy.
- Error categories are bounded and do not retain input text. This sanitization
  does not alter which wire inputs are accepted.

`fixtures/v1/cases.json` freezes acceptance and normalized JSON against the original
host. Boundary tests additionally exercise counts, depth, fields, bytes, paths and
URL restrictions. Do not regenerate expected outcomes to hide compatibility drift.

## Host-owned policy

Store-bound identity/grants, fuel/epoch/deadlines, cancellation, memory limits,
transactional quotas, SQLite, package staging and network transport remain host
responsibilities. Shared validation never makes downloaded data trusted: the host
must validate it independently on every relevant runtime/install boundary.

Schemas, package validation and the real compatibility canary arrive in the next
foundation tasks. No SDK, general author CLI, dynamic UI code or new capabilities
are provided here.
