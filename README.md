# rust_kv_store_distributed

A key-value store in Rust, built in phases — a Rust mirror of a C++
KVStore learning project. Not started yet; scaffold only.

## Build and run

```sh
cargo build
cargo run
cargo test
```

## Design

See `DESIGN.md` for the layering and the phase plan. Despite the name,
the distributed (multi-node) design is intentionally not decided yet —
phases 1-5 build a solid single-node engine first.
