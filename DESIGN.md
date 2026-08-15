# rust_kv_store_distributed — design

A key-value store in Rust, built in phases — mirrors the phase
structure of the C++ `kvstore` learning project. Demoable at the end
of every phase.

The name says "distributed," but the distributed design itself
(replication? consensus? sharding?) is **deliberately not decided
yet**. Phases 1-5 below build a solid single-node engine first, the
same way the C++ project did. See "Phase 6+" at the bottom.

## Core principle

The storage engine knows nothing about networking. `KvStore` is a
plain struct with `set/get/del`. Phase 1 drives it from a REPL, Phase
3 drives it from a socket. Neither one changes the engine.

```
          +-----------+
REPL  --> |           |
          |  KvStore  |   <- all logic lives here
socket--> |           |
          +-----------+
```

If adding the network layer later requires editing `kvstore.rs`, the
boundary was drawn wrong.

## Phases

| Phase | What                                        | Status      |
|-------|---------------------------------------------|-------------|
| 1     | KvStore + REPL, single-threaded              | done        |
| 2     | Tests, key expiry (TTL)                      | done        |
| 3     | TCP server (tokio), task per connection      | done        |
| 4     | Shared state across tasks (`RwLock`)         | not started |
| 5     | Benchmark harness                            | not started |
| 6+    | Distributed (multi-node) — not designed yet  | not started |

Do not start a phase before the previous one builds and runs.

## Command reference (Phases 1-2)

Commands, typed at a prompt:

```
SET <key> <value> [ttl_seconds]   -> OK | ERR invalid TTL
GET <key>                         -> <value> | (nil)
DEL <key>                         -> 1 | 0
EXISTS <key>                      -> 1 | 0
PING                              -> PONG
QUIT                              -> exits
```

- Keys and values are `String`.
- Values are single tokens for now (no spaces). Quoting comes later.
- Storage is `HashMap<String, Entry>`, where `Entry` holds a value and
  an optional expiry (`std::time::Instant`).
- `ttl_seconds` must be a positive integer. A plain `SET` (no ttl)
  always replaces the whole entry, clearing any TTL the key had
  before.
- `GET`/`EXISTS`/`DEL` treat an expired key as absent. `GET`/`EXISTS`
  physically purge it the moment they notice.
- No persistence.

## Phase 3+ (sketch only — flesh out when we get there)

- TCP server via `tokio`, `127.0.0.1` by default, same line-based
  framing idea as the C++ server (accumulate bytes per connection,
  split on `\n`, feed each line through the same command dispatch the
  REPL uses).
- One `tokio` task per connection instead of one OS thread per
  connection — same shape, different primitive.
- Shared `KvStore` behind a lock (`tokio::sync::RwLock` if the lock
  needs to be held across an `.await`, `std::sync::RwLock` if it
  doesn't — decide this when Phase 4 actually starts, not now).

## Layout

Rust doesn't split declaration/definition across header/source files
the way C++ does, so there's no `include/` — one module per concern
instead:

```
src/main.rs      entry point
src/kvstore.rs   engine (struct + impl)                    [Phase 1]
src/repl.rs      line parsing + command dispatch            [Phase 1]
src/server.rs    TCP accept loop, byte-stream framing       [Phase 3]
tests/           integration tests via `cargo test` — built into the
                 toolchain, no external framework, same spirit as the
                 C++ project's hand-rolled harness
benches/         throughput benchmark as a plain binary, not the
                 `criterion` crate — mirrors bench/bench.cpp
```

None of the module files above exist yet — they get created when that
phase actually starts.

## Deliberately deferred

Recorded so they are choices, not oversights:

- Concurrent read scaling, locking strategy in general — same open
  question the C++ project has (shared-lock-then-upgrade vs sharding),
  revisit once Phase 4 has a working baseline to measure.
- Values containing spaces or newlines.
- Persistence. Everything is lost on exit.
- Proactive eviction. Expired keys are only purged lazily.
- Binary-safe protocol.
- The entire distributed-systems design. Not decided: replication
  model, consistency guarantees, node discovery, failure handling.
  Revisit once Phases 1-5 are solid.
