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
closed until validation tooling, CI and repository review gates are established.

Plugin authors keep source code in their own repositories and publish
`.chronlyt-plugin` artifacts through their GitHub releases. This repository will
hold registry metadata and the generated catalog, not community plugin source.

The registry foundation is being implemented in stages. Public contracts,
validation tooling and one compatibility example will follow; no SDK, general
author CLI or plugin runtime is provided by this bootstrap.
