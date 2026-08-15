# Phase 6 trade-offs

Every design decision in `design.md` gave something up to get
something else. This is where that's argued out explicitly, instead
of left implicit in the architecture.

## The frame: CAP, informally

CAP theorem says: when a network partition happens (some nodes can't
talk to others — and on a real network, eventually one will), a
distributed system has to choose between staying **available**
(every node still answers requests) and staying **consistent** (every
node gives the same answer, as if there were only one copy of the
data). It can't fully have both during the partition.

Phase 6 is unambiguously an **AP** system: a follower cut off from the
leader keeps answering `GET`s with whatever it last had, rather than
refusing to answer until it's sure it's caught up. That's not a
missed requirement — it's decision #3 below, made on purpose.

## Decision 1 — replication, not sharding or Raft, first

**Gained:** the smallest possible step that's still genuinely
distributed. Replication alone introduces failure detection,
consistency lag, and the leader/follower split — the concepts that
recur in almost every harder distributed system — without also
requiring routing logic (sharding) or a correctness-critical protocol
(consensus) at the same time.

**Given up:** neither more capacity (sharding's job) nor stronger
consistency guarantees (Raft's job) exist yet. A single leader is
still the ceiling on write throughput and the single point of failure
for writes.

**The harder answer:** build all three at once, the way a production
system eventually needs to. Correct for a real system; the wrong
first step for learning any one of them — each one is easier to
reason about in isolation, and mistakes in one don't hide inside
mistakes in another.

## Decision 2 — async replication, not sync or quorum

**Gained:** the leader's write latency is exactly what it was in
Phase 4 — one local `dispatch`, one lock, one reply. Nothing a
follower does can slow down or block a client. Simple to reason
about: "the leader always makes progress" has no exceptions.

**Given up:** durability under leader failure. A client can receive
`OK` for a write that's already lost, if the leader crashes before
that write reaches even one follower. There's no way for a client to
know, at `OK` time, whether the write "really" survived — because in
this design, it might not have.

**The harder answer — sync replication:** the leader waits for at
least one follower to ack before replying `OK`. Buys a durability
guarantee ("if you got `OK`, at least 2 copies exist") at the cost of
every write now waiting on a network round-trip, and a new failure
mode (what does the leader do if the one follower it's waiting on is
down — block forever? time out and reply anyway, silently
degrading to async?).

**The harder-still answer — quorum (`W` acks out of `N` replicas):**
generalizes sync replication to tolerate some followers being down
without blocking writes entirely, at the cost of needing to define
`R` (read quorum) and `W` (write quorum) and reason about `R + W > N`
to guarantee a read sees the latest write. This is exactly the kind
of trade-off dial systems like Cassandra or DynamoDB expose to
operators — worth knowing exists, out of scope for a first
replication phase.

## Decision 3 — followers serve reads (and can be stale)

**Gained:** read throughput and locality that scale with the number
of followers — genuinely useful, and the concrete, observable
teaching artifact this design is built around (the `OFFSET` command
in `design.md` §6 exists specifically to make this decision visible).

**Given up:** the guarantee that a `GET` reflects the most recent
`SET`, if they hit different nodes. A client can `SET x 1` on the
leader, then `GET x` on a follower and see the old value, or nothing.
This is *eventual* consistency: the follower will converge, but "when"
is not part of the contract — only "OFFSET is monotonically
increasing and eventually matches the leader's" is.

**The harder answer — route reads to the leader too:** trivially
consistent (there's only ever one copy being read), but then
followers exist purely for failover-readiness, not for scaling reads
— the "pure standby" alternative this project's brainstorming
explicitly considered and didn't pick.

## Decision 4 — no automatic failover

**Gained:** correctness is easy to argue about, because there's
never more than one node that thinks it's the leader. No election
protocol, no split-brain risk, no term numbers, no "what if two nodes
both got votes" edge cases.

**Given up:** availability for *writes* specifically, when the leader
dies. Reads can keep working (stale, from followers); writes stop
until a human intervenes.

**The harder answer — leader election (Raft):** nodes vote, a
majority elects a new leader, and a "term" number invalidates the old
leader if it comes back online confused about its own status. This is
real, substantial complexity — not because the idea is hard to state,
but because getting split-brain-proof election right under every
interleaving of crashes and network delays is famously easy to get
subtly wrong. That's precisely why it's scoped as a separate,
dedicated phase rather than folded into Phase 6 — see
`learning-roadmap.md`.

## Honest gaps (not virtues, just costs, stated plainly)

- **A follower that was ever down doesn't resync.** It resumes
  applying whatever writes arrive *after* it reconnects; whatever it
  missed while down is gone until a human rebuilds it from scratch.
  A real system needs either a persisted, replayable log (what Raft's
  log gives you) or a snapshot-transfer mechanism on reconnect.
  Neither exists here.
- **The replica link isn't authenticated**, only sanity-checked
  against a configured address. Fine on `127.0.0.1` for learning;
  not a design that should touch an untrusted network.
- **No ordering guarantee across followers**, only per-follower
  ordering relative to the leader. Two followers can be at different
  offsets from each other at any instant — there's no mechanism (and
  no goal) for followers to agree with *each other*, only with the
  leader's history.
- **No read-your-writes.** A client that just wrote to the leader has
  no way to guarantee its own next read (even to the same follower)
  reflects that write, unless it manually checks `OFFSET` first. Some
  real systems solve this with "sticky sessions" or client-tracked
  offsets; Phase 6 does neither.
