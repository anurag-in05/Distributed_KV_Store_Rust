# Learning roadmap: distributed systems concepts

Not scoped to this project specifically — this is the general
map of ideas Phase 6 sits inside. Each concept notes where it showed
up (or was deliberately avoided) in Phase 6, so it stays concrete
rather than a bare glossary, plus a next step if you want to go
deeper than reading about it.

## Foundational (Phase 6 already exercises these)

**CAP theorem.** Partition tolerance is mandatory on a real network,
so the actual choice is availability vs. consistency during a
partition. Phase 6 picked AP explicitly — `tradeoffs.md` walks through
why. Understanding CAP well means being able to say, for any system,
*which* operations are available during a partition and *which*
consistency guarantee is given up — "AP" as a one-word label is a
start, not the whole answer.

**Consistency models**, roughly strongest to weakest:
- *Strong/linearizable* — every read sees the latest committed write,
  system-wide, as if there were one copy. What sync/quorum
  replication or Raft can provide.
- *Causal* — if write A "happened before" write B (e.g. B read A's
  result), every node sees A before B. Doesn't require a total global
  order, just respects cause-and-effect. Phase 6 doesn't provide this
  either — a follower can apply writes it received, but there's no
  mechanism tracking causality across clients.
- *Eventual* — the model Phase 6 implements. All replicas converge
  *eventually* if writes stop; no promise about *when*, and no
  ordering guarantee relative to a client's own prior actions
  (that's the "no read-your-writes" gap in `tradeoffs.md`).

Next step: work through a few `GET`/`SET` sequences across leader and
follower by hand and classify what each would return under strong vs.
eventual consistency — the difference stops being abstract fast.

**Replication strategies.**
- *Leader-follower (primary-backup)* — what Phase 6 builds. One
  designated writer, N copies.
- *Leaderless (quorum-based)* — any node can accept a write; a client
  writes to `W` replicas and reads from `R` replicas, and `R + W > N`
  guarantees the read set and write set overlap by at least one node,
  so a read is guaranteed to see the latest write. Cassandra and
  DynamoDB work this way. No single point of failure for writes,
  at the cost of needing conflict resolution (see below) when two
  clients write concurrently, since there's no leader to serialize
  them.

Next step: design (on paper) what Phase 6 would look like with no
leader — specifically, what happens when two clients concurrently
`SET x` on two different nodes. That question is *why* leaderless
systems need the next concept.

## Where Phase 6 stops short

**Consensus algorithms (Raft, Paxos).** The mechanism that lets a
cluster agree on one ordered sequence of writes and survive a leader
crashing *without* losing any acknowledged write or risking two nodes
both believing they're leader ("split-brain"). Core ideas: a *term*
number that increases every election so stale leaders recognize
they've been superseded; a majority vote to become leader; a
replicated *log* where an entry only counts as committed once a
majority of nodes have it. Phase 6's async replication has none of
this — it accepts the "can lose a write" and "no auto-failover"
costs specifically to avoid this complexity.

Next step, if you want the deepest single upgrade to this project:
implement Raft leader election as a Phase 7. Read the Raft paper
(Ongaro & Ousterhout, *"In Search of an Understandable Consensus
Algorithm"* — it was written specifically to be more approachable
than Paxos) before writing code; the subtlety is almost entirely in
the edge cases around concurrent elections and log conflicts, not in
the happy path.

**Sharding / partitioning.** A different problem from everything
above — not "how do copies of the same data stay in sync" but "how do
we split data that's too big or too hot for one node." Core idea:
*consistent hashing* — instead of `hash(key) % N` (which reshuffles
almost everything when `N` changes), map both nodes and keys onto a
hash ring, and a key belongs to the next node clockwise from it. Adding
or removing a node only moves the keys near it on the ring, not
everything.

Next step, as a Phase 8: shard by key across the *existing* Phase 6
replication units — i.e., each shard is itself a leader + followers.
That's genuinely how systems like MongoDB and CockroachDB are
structured (sharding and replication as two independent, composed
layers, not one mechanism doing both jobs).

## Supporting concepts, worth knowing exist

**Failure detection strategies**, in increasing sophistication:
fixed-timeout heartbeats (what Phase 6 uses — simple, but a single
slow response looks identical to a dead node); phi-accrual detectors
(track the *distribution* of heartbeat intervals and flag a node as
suspect based on how anomalous the current gap is, rather than a
hard cutoff — used by Cassandra); gossip protocols like SWIM (nodes
tell each other what they've observed, so failure information
propagates without every node needing a direct connection to every
other node — necessary once cluster size makes all-to-all heartbeats
too expensive). Phase 6's static, direct, star-topology heartbeats
are the simplest end of this spectrum on purpose.

**Logical and vector clocks.** Wall-clock time can't order events
across machines (clocks drift, and Phase 6 doesn't even try to use
timestamps for ordering — it uses the offset counter instead, which
is a logical clock in miniature). A *Lamport clock* gives a
total order consistent with causality using a simple counter. A
*vector clock* (one counter per node) can detect when two writes were
truly *concurrent* — genuinely unordered — rather than one causing
the other, which single-counter schemes can't distinguish. This
matters the moment you have leaderless writes (above) and need to
detect and resolve conflicting concurrent updates.

**Idempotency and delivery semantics.** Phase 6's replica link has no
retry-with-dedup logic, so it's implicitly *at-most-once* per
attempt (a dropped write from a full channel is just gone, not
retried — see `tradeoffs.md`). *At-least-once* delivery (retry until
acked, possibly applying twice) requires the receiving operation to
be *idempotent* — applying it twice has the same effect as once.
`SET x 1` already is idempotent for free; something like `INCR x`
would not be, and would need a dedup mechanism (e.g. a per-write id)
to be replayed safely. *Exactly-once* is the hardest and most
expensive to actually guarantee end-to-end; most real systems build
at-least-once + idempotency instead of chasing true exactly-once.

**Quorum reads/writes (`R + W > N`).** Already introduced above under
leaderless replication — worth calling out on its own because it's
the one formula that shows up in most quorum-based system designs
and is worth being able to derive, not just quote: if a write touches
`W` nodes and a read touches `R` nodes out of `N` total, `R + W > N`
guarantees at least one node is in both sets, so the read is
guaranteed to see the latest acknowledged write. Tune `R` and `W` to
trade read latency against write latency against consistency
strength.

## Suggested order if starting from here

1. Sit with `tradeoffs.md` until the AP choice and its consequences
   feel concrete, not just quotable.
2. Sketch (on paper, no code) what a `STATUS`/`OFFSET`-style
   introspection command would need to look like for a *quorum*
   system instead of Phase 6's async one.
3. If you want hands-on depth: implement Raft leader election as
   Phase 7 on top of this codebase — it's the single concept here
   with the most subtlety and the most payoff for actually building
   it rather than reading about it.
4. If you want breadth instead: read about how one real system
   (Cassandra for leaderless/quorum, etcd or CockroachDB for
   Raft-based, Redis Cluster for sharding + per-shard replication) is
   actually built, and map its choices back onto this document's
   framing — every one of them made the same five kinds of decisions
   Phase 6 made, just with different answers.
