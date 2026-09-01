# Publishing workflow (foundation scope)

This foundation does not publish a plugin for you. Community intake stays closed
until the trusted workflow/tooling and external review gates are ready.

1. Keep source in your own repository. Target the canonical
   `chronlyt:plugin@1.0.0` WIT and existing manifest/UI contracts; a host engine
   version is not the public compatibility contract.
2. Build a compatible component and package it under the existing archive rules.
   No new SDK, general build/pack CLI, template or WASI adapter is implied here.
   The foundation's [minimal canary](../examples/minimal-page/README.md) is a
   source-built compatibility example, not an SDK or submission template.
3. Validate locally with the canonical contracts/pure package validator. Publish
   the `.chronlyt-plugin` as an immutable versioned GitHub release artifact only
   when you intentionally choose to publish it. This repository never executes
   an author's release scripts during registry validation.
4. Add/update the source record and explicit registry timestamp, then generate
   `catalog.json` using the trusted baseline as documented in [registry.md](registry.md).
   Authors do not manually calculate catalog SHA-256 or size. Commit both source
   and generated catalog to the same PR when submission is open.
5. Trusted registry-data validation independently fetches, checks and regenerates
   the catalog. Ordinary unprivileged candidate-code tests are a separate trust
   boundary. Sensitive validator/workflow/contracts changes must be reviewed and
   available on protected main before submissions can rely on their behavior.

Never replace an existing version's artifact bytes. Changed packages require a
forward SemVer release and, in Chronlyt, explicit approval for new permissions.
Registry acceptance is not code signing or a `verified` badge. Chronlyt must
independently validate every downloaded package and enforce host-owned grants.
