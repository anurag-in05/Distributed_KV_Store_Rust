# Context for Claude Code

Read `DESIGN.md` first. It defines the phases and the current scope.

## About this project

A learning project, mirrored from a C++ KVStore (a sibling project).
The author has written the C++ version already and knows the domain
(KV store, REPL, TCP server, locking) but this is their first project
applying Rust — ownership/borrowing, `Option`/`Result`, and eventually
`tokio` — to it. Explanations matter more than finished code.

## Working rules

- **Stay inside the current phase.** `DESIGN.md` marks it. Do not add
  `tokio`, networking, or persistence early, even if trivial.
- **Do not write large blocks of code unprompted.** Suggest the
  change, explain the reasoning, wait to be asked.
- **Explain the Rust-specific choice** whenever one comes up: why
  `&str` over `String` in a signature, why `Option` over a sentinel
  value, ownership vs borrowing, why a `Result` over panicking, why a
  trait over duplicating code. Short — one or two sentences.
- **Prefer the standard library** over external crates. `tokio` is the
  one planned exception, reserved for the concurrency/networking
  phases (3+) — don't add it to `Cargo.toml` before that phase needs
  it.
- **No premature abstraction.** No traits with a single implementor,
  no generics until something needs to be generic.
- If a change would blur the KvStore / transport boundary, say so
  instead of doing it.

## Build

```sh
cargo build
cargo run
cargo test
```

## Style

- Rust 2024 edition. Standard library only, except `tokio` from
  Phase 3 onward.
- 4-space indent (`cargo fmt` default), `snake_case` for
  functions/variables, `PascalCase` for types. Plain Rust visibility
  (private by default) — no `m_` prefix, that's a C++ convention that
  doesn't carry over.
- Run `cargo fmt` before considering a change done.
