# Chronlyt-Plugins

The public registry and contract home for Chronlyt Plugin System v1.

## Registry foundation

The root `catalog.json` is a valid empty v1 catalog. An empty plugin list means
that no plugins are published here yet; it is not an error or a demo catalog.
Its explicit generation timestamp is recorded in `registry/metadata.json`.

Chronlyt reads the root catalog from:

`https://raw.githubusercontent.com/n3d7/Chronlyt-Plugins/main/catalog.json`

Local changes do not update that endpoint. They must first be reviewed and
published to `main` by an authorized maintainer. Community submissions remain
closed until the separate registry intake and review gates are established.

Plugin authors keep source code in their own repositories and publish
`.chronlyt-plugin` artifacts through their GitHub releases. This repository will
hold registry metadata and the generated catalog, not community plugin source.

The foundation contains canonical public contracts, pure validation tooling and
a source-built [minimal compatibility canary](examples/minimal-page/README.md).
The canary proves the v1 build/package boundary; it is not an SDK, template,
published registry plugin or plugin runtime.
