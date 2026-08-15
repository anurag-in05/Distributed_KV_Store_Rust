# Phase 6 design — async leader-follower replication

Architecture for the decisions recorded in `README.md`. File paths
and signatures below refer to the actual current code, not a
hypothetical one — this is meant to be buildable as written, not just
plausible.

## 1. Role model and CLI

A node is either a leader or a follower — a genuinely different
runtime behavior, decided once at startup, never changing while the
process runs:

```rust
// src/topology.rs (new)
pub enum Role {
    Leader { peers: Vec<String> },   // follower listen-addresses
    Follower { leader_addr: String },
}

pub struct NodeConfig {
    pub listen: String,
    pub role: Role,
}
```

```
# leader
cargo run --bin server -- --role leader --listen 127.0.0.1:7878 \
    --peers 127.0.0.1:7879,127.0.0.1:7880

# follower
cargo run --bin server -- --role follower --listen 127.0.0.1:7879 \
    --leader 127.0.0.1:7878
```

Parsed by hand from `std::env::args()` — no `clap`. This project has
one external dependency (`tokio`) by design; a two-flag CLI doesn't
earn a second one.

`Role` being an enum with exactly two variants is *not* the
"premature abstraction" CLAUDE.md warns against — it's not
speculative, it models two behaviors that already, concretely,
differ (a leader fans out writes and accepts client writes; a
follower doesn't). The abstraction-to-avoid would be a `trait Node`
with `LeaderNode`/`FollowerNode` structs implementing it before
there's a second implementation of anything shared.

No separate node-id concept. The listen address doubles as identity
everywhere — logs, the handshake payload, the peer-status map key.
Introducing a synthetic id would be complexity with no current use.

## 2. Replication transport: same port, a handshake line

The leader dials each follower with a plain `TcpStream::connect` —
the same thing `tests/server.rs`'s `Client` already does — and the
first line it sends marks the connection as internal:

```
REPLICA 127.0.0.1:7878\n
```

The follower's connection handler checks the first line before doing
anything else. If it matches `REPLICA <addr>`, that one connection
becomes a trusted replica link for its whole lifetime, distinct from
every ordinary client connection on the same listener.

**Why one port, not two:** it reuses the exact accept loop and
line-reading code `handle_connection` already has — no second
`TcpListener`, no second thing to configure or firewall. It's also
not just a shortcut: Redis and Kafka both replicate over the same
port clients use, for the same reason. The honest cost: distinguishing
replica from client is a string check, not authentication — anything
that can open a TCP connection to a follower can *claim* to be the
leader. Mitigated cheaply, not securely: a follower rejects a
handshake whose claimed address doesn't match its configured
`--leader`. Good enough for a loopback teaching exercise, explicitly
not for anything else — see `tradeoffs.md`.

Only one live replica link is expected per follower (single leader,
star topology). A second handshake attempt is rejected and logged.

## 3. Where this touches the existing code

| File | Change |
|---|---|
| `src/kvstore.rs` | **None.** Still network- and role-ignorant. |
| `src/repl.rs` | One small addition: `pub(crate) fn is_write_command(line: &str) -> bool` — matches `SET`/`DEL`. `dispatch`'s signature is unchanged. |
| `src/topology.rs` | New. `Role`, `NodeConfig`, CLI parsing (§1). |
| `src/replication.rs` | New. `ReplicationHub` — leader-side fan-out state (§4), peer status (§5). |
| `src/server.rs` | `run` takes a `NodeConfig` instead of a bare address string; `handle_connection` grows a role-aware branch (handshake check, write-gating, offset interception). |
| `src/bin/server.rs` | Parses CLI args via `topology::parse_args` instead of a hardcoded address constant. |

`is_write_command` belongs in `repl.rs`, not duplicated in
`server.rs`, because it's still *protocol grammar* — the same kind of
knowledge `dispatch`'s match arms already encode (which verbs exist).
It is not policy about roles; a follower using it to reject a write
is a `server.rs` decision, not a `repl.rs` one. If a future command
like `INCR` needs the same treatment, there's exactly one place that
knows which verbs mutate.

This is the load-bearing point for the "does this blur the
KvStore/transport boundary" question CLAUDE.md asks about: it
doesn't. Every new concept in Phase 6 — role, handshake, replicated
writes, offsets — lives in the transport layer or in the two new
modules. `dispatch`'s contract, `(store: &mut KvStore, line: &str) ->
Option<String>`, is unchanged.

A follower rejects a client write without even taking the lock:

```rust
if is_write_command(line) && role.is_follower() {
    reply(format!("ERR not leader, try {}", role.leader_addr()));
    continue;
}
```

## 4. Write path: ordering without a new lock

Phase 4 wraps `KvStore` in `Arc<std::sync::RwLock<KvStore>>` for an
unrelated reason — `get`/`exists` take `&mut self` because lazy TTL
purging can mutate on a read, so even reads need the write lock.
A side effect: every command on the leader is already serialized into
one total order by that lock. Phase 6 spends that order rather than
adding a new synchronization mechanism:

```rust
let response = {
    let mut guard = store.write().unwrap();
    let r = dispatch(&mut guard, line);
    if role.is_leader() && is_write_command(line) && !r.starts_with("ERR") {
        repl.enqueue(line); // still inside the critical section
    }
    r
};
```

Each follower has one dedicated background task owning that
follower's connection, draining a bounded `mpsc::Receiver<String>`
(capacity ~1024) in strict FIFO order. Connection handlers call
`try_send` — synchronous, never `.await`ed — while still holding the
write guard.

**Why not `tokio::spawn` a task per write** (the seemingly-simpler
option): two independently-spawned tasks writing to the *same*
follower socket can interleave partial writes (wire corruption), and
nothing guarantees they reach the follower in the order the leader
applied them — `KvStore` has no per-key version or timestamp to
arbitrate a reordering, so `SET x 1` then `SET x 2` arriving as `2`
then `1` leaves the follower silently, permanently wrong on `x`. A
single dedicated sender per follower, fed in commit order, is what
prevents that — not a nice-to-have, a correctness requirement.

**Why enqueue inside the lock, not after releasing it:** if the
enqueue happened after the lock was released, two concurrent writers
could apply in order A→B but enqueue in order B→A depending on task
scheduling. Enqueuing before releasing pins replication order to
application order.

**Why bounded + `try_send`, not unbounded + `.send().await`:** the
leader must never block a client on a slow or dead follower. A full
channel means `try_send` fails immediately; that write is dropped for
*that follower only*, which is marked `Unreachable` (§5). The client
already got its reply independent of any of this. An unbounded
channel would avoid the drop but trade it for unbounded memory growth
against a follower that never comes back — worse, not better. The
honest consequence, stated plainly: a follower down long enough to
overflow its channel doesn't just lag, it silently drops writes and
needs a full rebuild to recover. Phase 6 has no resync mechanism —
see `tradeoffs.md`.

## 5. Failure detection: heartbeats, no action

Each per-follower task also runs a `tokio::time::interval` (e.g. 1s),
selecting between "a write is queued" and "the interval ticked" via
`tokio::select!`. When idle, it sends `PING` down the replica link.
The follower's replica-link loop can literally call the existing
`dispatch(&mut store, "PING")` — `PONG` already exists, zero new
logic.

No response within ~2-3 intervals, or a socket error, marks that peer
`Unreachable` in `ReplicationHub`'s status map and logs it. The task
then retries the connection on a fixed delay; success flips the
status back and logs a reconnect. Symmetric on the follower side: a
closed replica-link socket is handled by the same "connection closed"
path every `handle_connection` already has for any disconnect.

**What happens on `Unreachable`, stated plainly: nothing beyond the
log and the status flag.** No election, no promotion — there's no
quorum concept in async replication to begin with, since the leader
never waited for any ack. Not built: gossip, phi-accrual detection,
backoff/jitter tuning, multi-hop propagation. Fixed interval, fixed
timeout, fixed retry — that's the whole mechanism.

## 6. Making staleness observable: the offset counter

The single highest-value piece for the stated teaching goal. A
`AtomicU64` on the leader increments once per successfully-applied
write, in the same critical section as the mutation (riding the
existing lock again, no new synchronization). Each replicated write
carries that number: `REPLICATE 42 SET foo bar` — a wire-format
detail that only ever appears on replica links, never sent to a real
client, so `dispatch`'s protocol is untouched.

A follower tracks `last_applied_offset`, bumping it after applying
each `REPLICATE` line, and replies `ACK 42` back up the replica link.
A new client-facing command, `OFFSET`, returns a node's current
counter as a plain integer — intercepted directly in
`handle_connection`, deliberately *not* added to `dispatch`'s match
table (adding it there would mean threading role/offset state into
`dispatch`'s signature, exactly the boundary CLAUDE.md protects).

This makes the core lesson concrete instead of hand-wavy:

```
leader>   SET foo bar     -> OK      (leader OFFSET: 5 -> 6)
follower> OFFSET          -> 5       (hasn't caught up yet)
follower> GET foo         -> (nil)   (stale!)
...
follower> OFFSET          -> 6       (caught up)
follower> GET foo         -> bar     (converged)
```

If a follower's `dispatch` call for a leader-originated write ever
returns an `ERR`, that means the follower has diverged from a leader
that already validated the write — a real bug, not expected
behavior. Log it loudly (`"follower diverged: could not apply offset
42: <reason>"`) as a defensive invariant check.

## 7. Testing

New `tests/replication.rs`, same hand-rolled pattern as
`tests/server.rs` — a `Client` struct over `OwnedReadHalf`/
`OwnedWriteHalf`, no external test crates, `start_leader`/
`start_follower` helpers mirroring the existing `start_server`.

| Test | What it proves |
|---|---|
| Replication converges | `SET` on leader; poll follower `OFFSET` (bounded retries) until it matches; `GET` on follower returns the value. |
| Followers reject writes | `SET` sent directly to a follower gets `ERR not leader, try <addr>`; follower's store unchanged. |
| Concurrent writes stay ordered | Many clients `SET` distinct keys on the leader concurrently (extends the existing `concurrent_writes_from_many_connections_all_land` pattern); poll `OFFSET` to converge; assert follower's keys match the leader's exactly. |
| Failure detection | `JoinHandle::abort()` a follower's task mid-test (already-available `rt` feature, no new dependency); assert the leader's status flips to `Unreachable` after the heartbeat timeout, via a small `STATUS` introspection command built the same way as `OFFSET`. |

**Deliberately not a test:** asserting a `GET` on a follower
*immediately* after a leader `SET` observes the pre-replication
value. On loopback, replication can complete in well under a
millisecond — a hard "must observe staleness" assertion is
inherently racy, i.e. actually flaky, not just theoretically so.
`OFFSET`-polling (a bounded *convergence* wait, not a race) is the
correct automated proxy for the same idea. Actually watching lag
happen is better done as a manual two-terminal walkthrough than as a
CI assertion — adding a synchronization hook to production code just
to make a race artificially observable in a test would be complexity
serving the test, not the system.

## 8. Explicitly not built in Phase 6

Conflict resolution (impossible by construction — followers never
accept writes), automatic failover / leader election, dynamic
membership or gossip, log persistence or resync-after-restart,
quorum/synchronous replication or tunable durability, sharding,
full Raft, replica-link authentication, a binary/efficient wire
format, chained (multi-hop) replication, read-your-writes or session
consistency guarantees. Each is discussed — not just listed — in
`tradeoffs.md`.
