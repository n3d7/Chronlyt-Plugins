# Minimal compatibility canary

This is a source-built compatibility canary for the canonical
`chronlyt:plugin@1.0.0` world `chronlyt-plugin`. It renders one declarative page
and handles one `ping` action. It requests no permissions and makes no host or
WASI calls.

The example is not an SDK, template, registry submission or published plugin.
Its generated bindings consume the canonical public WIT through
`chronlyt_plugin_contracts::WIT_SOURCE`; there is no second maintained WIT copy.

Build and validate it from the repository root:

```sh
cargo build --locked --release -p chronlyt-plugin-example-minimal \
  --target wasm32-unknown-unknown
CHRONLYT_CANARY_CORE_WASM="$PWD/target/wasm32-unknown-unknown/release/chronlyt_plugin_example_minimal.wasm" \
  cargo test --locked -p chronlyt-plugin-validator --test canary_package -- --ignored
```

The second command componentizes the source-built core module without a WASI
adapter, validates the outer Component Model imports and canonical Chronlyt ABI,
then writes transient proof outputs under `target/canary/`.
