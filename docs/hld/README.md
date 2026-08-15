# Phase 6 HLD — distributed replication

`DESIGN.md` marked Phase 6+ (distributed) as "not designed yet." This
folder is that design. It covers **replication only** — not sharding,
not full consensus — chosen as the smallest step that's still
genuinely distributed and still teaches the concepts that matter most.

This is a design document, not implementation. Nothing in `src/`
changes because of this folder. Implementation is a separate, later
phase, planned on its own once (and if) there's an appetite to build
it.

## What Phase 6 is

One node is the **leader** (accepts writes), N nodes are
**followers** (replicate the leader's data, also serve reads). Five
decisions, made deliberately and documented with their trade-offs:

1. **Replication only** — not sharding (a scale problem), not full
   Raft (a stronger-consistency problem). Both are plausible later
   phases; see [`learning-roadmap.md`](./learning-roadmap.md).
2. **Async replication** — the leader applies a write and replies to
   the client without waiting for any follower. Simpler, faster, but
   a write can be lost if the leader crashes before replicating it.
3. **Followers serve reads** — `GET`/`EXISTS` can hit any node,
   `SET`/`DEL` only the leader. This means a follower can serve
   *stale* data. That's not a bug being tolerated — it's the point:
   this is where "eventual consistency" stops being a phrase and
   becomes something you can watch happen.
4. **No automatic failover** — a dead leader doesn't trigger an
   election. Recovering means an operator steps in. Automatic
   failover needs consensus (terms, votes, a replicated log) to do
   safely, which is exactly what a Raft phase would add.
5. **Static peer config** — nodes are told their role and peers at
   startup via CLI flags. No dynamic membership, no gossip.

## Where to read next

- [`design.md`](./design.md) — the architecture: how nodes find each
  other, how writes replicate, where this hooks into the existing
  code, how it'd be tested.
- [`tradeoffs.md`](./tradeoffs.md) — each decision above, argued out:
  what it buys, what it costs, and what the "harder" answer would
  have looked like.
- [`learning-roadmap.md`](./learning-roadmap.md) — the broader
  distributed-systems concepts this phase touches (and deliberately
  avoids), with suggested next steps if you want to go further.

## Explicitly out of scope for Phase 6

Conflict resolution (impossible by construction — only the leader
writes), quorum/synchronous replication, log persistence or
resyncing a follower that was down, authenticating the replica link,
chained replication, dynamic membership, read-your-writes guarantees.
Each of these is addressed directly in `tradeoffs.md` rather than
silently skipped.
