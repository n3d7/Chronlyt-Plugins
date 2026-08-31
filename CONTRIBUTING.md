# Contributing

Community plugin intake is **closed during Registry Foundation v1 bootstrap**.
The local foundation is not authorization to publish or change repository rules.
See [registry source/commands](docs/registry.md), [author workflow](docs/publishing.md)
and [security invariants](SECURITY.md).

Keep plugin source in the author's repository and immutable `.chronlyt-plugin`
artifacts in versioned GitHub releases. Once intake opens, a registry PR changes
source metadata and its deterministically generated root catalog together.
Do not submit manual digests/sizes, trusted badges or executable registry tools.

Changes to workflows, canonical contracts/WIT, validators, generator, fixtures,
dependencies/toolchain and review policy require protected/code-owner review.
Such code must become reviewed main code **before** a data submission relies on
the new behavior. A candidate's green check is not its own trusted validation.

Local code checks:

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo run --locked -p chronlyt-registry -- schemas --check
```

No SDK, general CLI, scaffolding or new capabilities are part of this milestone.
The real minimal compatibility canary is Task 6; it must be built/tested before
the complete foundation milestone is declared finished.
