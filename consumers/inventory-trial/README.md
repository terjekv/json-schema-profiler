# Inventory profiler trial

A standalone synthetic application exercising `json-schema-profiler` through its
public API. It owns records, snapshot identity/revisions, and schema application.
It is intentionally unpublished and separate from the library workspace.

Run from the repository root:

```sh
cargo run --manifest-path consumers/inventory-trial/Cargo.toml --locked
cargo test --manifest-path consumers/inventory-trial/Cargo.toml --locked
```

See the [trial report](../../docs/consumer-trial.md) for scenarios, limits,
measurements, and follow-up findings. Source data is synthetic and fixed at three
records. This demonstration does not implement a production storage or
authorization layer, nor bounded streaming parsing.
