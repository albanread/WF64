# What WF64's Forth side needs from paged_gc

A working note from the Forth team to the GC team, written part-way
through landing V2 of `docs/gc_design.md`.  This is the punch list of
things we are reaching for that either don't exist yet, work but feel
underspecified, or work but lock us into more boilerplate than seems
necessary.  None of it blocks V2 stage B — we have workarounds — but
all of it would simplify the V2c → V5 trajectory.

Status of WF64's integration as of this writing: V1a–V1c (substrate +
HEAPPTR + forget integration + nil-deref + dedicated throw codes) and
V2 stage A (auto-GC trigger + `gc-cycle` counter) are landed and
green.  Heap is a `PageHeap<Wf64Layout>` in a `thread_local`, lazily
initialised at 64 MB reservation, root set is the contiguous HEAPPTR
region in the user area, walked precisely by `evac.visit_cell` on
every collection.

The items below are roughly ordered by when WF64 will hit them.

## 1. Static-region card table — needed for V2 stage C

Right now `collect_minor` rescans every cell in `[HEAPPTR_BASE,
HEAPPTR_NEXT)` regardless of whether anything was actually written
into it since the last collection.  At V1 sizes (512 slots, 4 KB)
this is free.  At realistic working-set sizes (10K+ slots after a
session has been running for a while), it stops being free.

paged_gc already has `collect_minor_with_static(static_cards,
static_base, static_cells, pin_stack_ranges)` — exactly the right
shape.  But the binding-side requirements are non-trivial:

- We have to allocate and maintain a `CardTable` for the HEAPPTR
  region ourselves.
- We need a barrier word — call it `!heapptr` in WF64-speak — that
  dirties the appropriate card on every write into a HEAPPTR slot.
- We need a way to roll the card table back when `forget` rewinds
  `HEAPPTR_NEXT` (per V1c semantics).

**Ask:** A higher-level convenience like
`PageHeap::register_static_root_region(base, size_cells)` that hands
back an opaque handle, with `heap.write_static_slot(handle, offset,
val)` doing the card-mark internally.  Or, failing that, a short
worked example in `newgc-core`'s docs showing the
`collect_minor_with_static` flow with a single static region — what
goes in the `CardTable`, how to size it, who owns the card storage,
when to call `card_table.reset()`.

We can build this on the existing primitives, but the existing
primitives feel like they were written for the coordinator
(multi-region, multi-stack) and the single-region static-root case
isn't the obvious first example.

## 2. User write-barrier API — needed for V3 (`vec-ref!`)

When V3 lands `vec-ref!  ( tagged handle index -- )` writing a
tagged pointer into payload cell `index` of a RefVec, that store
needs a write barrier whenever the target is tenured.  Without it,
a minor GC that walks only the HEAPPTR roots will miss young
objects reachable only via tenured-RefVec → young-X edges.

paged_gc has `mark_card_at(slot_addr: *const u8)` — exactly the
right primitive.  But:

- The doc-string says it short-circuits if `slot_addr < storage.base()`.
  Useful, but it means we need to know we're inside the heap before
  calling.  That's fine for `vec-ref!` (we just dereffed a tagged
  pointer to find the cell), but worth documenting as a contract
  rather than a comment.
- Is `mark_card_at` cheap enough to call unconditionally on every
  RefVec write, or should we gate it on "the containing object is
  in G1/Tenured"?  We don't currently have a fast way to ask "what
  generation is this cell in" from the binding side.

**Ask:** Confirm `mark_card_at` is the right hook for binding-driven
writes, and either (a) make it cheap-enough-to-call-unconditionally,
or (b) expose a `heap.generation_of(addr) -> Generation` so the
binding can avoid the call when the target is already G0.

## 3. Pin support for legacy interop — needed for V2s (strings)

`docs/strings_design.md` specifies `$>addr ( $ -- c-addr u )` as the
"alloca-style legacy-interop escape" for passing string bytes to
Win32 APIs like `MessageBoxA`.  The lifetime contract is "valid
until next allocation or collection."

That contract is enforceable if:

- The raw pointer is read just before the Win32 call.
- No allocation happens between read and call.
- No collection happens between read and call.

With auto-GC in V2, condition #3 is fragile — `should_collect()`
runs on every `vec-alloc-*`, so any allocation inside the
post-`$>addr` window could trigger a minor and move the string.
Large objects are pinned by paged_gc and large strings would be
fine; small strings can move.

**Ask:** Either:

(a) A scoped pin API: `heap.pin(addr, |pinned_addr| { ... })`.
    Within the closure, paged_gc agrees not to move the object.
    Cheap-to-call; tens of pin/unpin events per second would be
    plausible.

(b) Confirmation that "ensure no allocation between read and call"
    is the right contract and `$>addr` should be documented as a
    sharp tool.  In that world the binding doesn't need new API,
    just discipline.

Either is fine — we just need to know which world we're in before
strings ship.

## 4. Trigger configuration — minor convenience

`auto_gc_trigger_bytes` defaults to 8 MB and recomputes after each
collection as `current_alloc + max(8 MB, 0.5 * tenured_used)`.  This
is reasonable for vector workloads (our V1b 2 MB worked example
needed 4 of them to trip a collection — fine).  It's pessimistic
for small-object-heavy workloads (Cons cells, small strings) where
you'd want collections every few hundred KB.

There's no `with_auto_gc_trigger(bytes)` builder method or
`heap.set_auto_gc_trigger(bytes)` setter that we found.  Currently
the field is `pub(super)` so we can't tweak it from the binding.

**Ask:** Expose a setter (or builder) for `auto_gc_trigger_bytes`
and `gc_budget_min_bytes`.  We'd default-tune them at session boot
based on workload hints, or expose them as Forth user variables
like `GC-TRIGGER` if that turns out to be needed.

## 5. Heap stats for the harness — needed for V2 stage C tests

The current `gc-cycle` counter is enough to verify "a collection
happened."  To verify "a *minor* collection happened" or "a *major*
collection happened" the harness wants two separate counters.
Same for per-generation occupancy when we write tenure-promotion
stress tests.

paged_gc already tracks all of this internally — `tenured_used_bytes`,
`should_collect_major`, the cycle index inside `CollectResult`, etc.

**Ask:** A `heap.stats() -> HeapStats { minor_cycles, major_cycles,
g0_bytes, g1_bytes, tenured_bytes, bytes_alloc_since_gc, ... }` would
let the binding read out anything we want for tests.  Bonus: a
`stats.delta(prev_stats)` helper for "how much changed since last
checkpoint" — that's what most assertions actually want.

## 6. Heap clear vs heap drop — small ergonomics

`reset_wf_heap` currently drops the entire `PageHeap` and
re-initialises on next use.  That works (and the address-space cost
of reserving 64 MB on session start is low), but it does mean we
flush all paged_gc's bookkeeping — adaptive trigger sizing, etc. —
on every harness test reset.  Each test starts the auto-trigger
math from scratch.

**Ask:** A `heap.clear()` that drops all live objects but keeps the
reservation and resets the bookkeeping to "fresh as if newly
created."  Conceptually a no-op major-GC over an empty root set, but
the API surface lets us not couple "reset for tests" to "drop and
re-VirtualAlloc."

Low priority — current behaviour works.

## 7. Major-GC trigger awareness — for V2 stage C

We have `should_collect()` wired into `vec-alloc-*` and run a minor
GC on true.  We don't currently wire up `should_collect_major()`.
The doc suggests checking it as a separate condition (perhaps inside
the minor-GC routine, or after it) and promoting to major when
tenure pressure exceeds the 75% threshold.

**Ask:** Confirm the right pattern.  Specifically:

- Should the binding call `should_collect_major()` *every* time, or
  only after a `should_collect()` returns true?
- If a major is needed and we're inside an auto-trigger, should we
  do the major directly (skipping minor), or always minor-first?
- Is `collect_major` ever cheaper than expected when most G1/G0 is
  empty, or should we treat it as roughly proportional to
  `tenured_used`?

Answers go in a comment in `vec_alloc_floats_store` / wherever we
end up putting the trigger logic.

## 8. Cycle counter — feature alignment

We added a thread-local `WF_GC_CYCLES` counter incremented in
`collect_major`/`collect_minor` (V2 stage A) — exposed as the Forth
word `gc-cycle ( -- n )`.  paged_gc internally tracks a similar
notion (the promotion-on-cycle-3 logic in `collect_minor`).

**Ask:** Is there a public accessor on `PageHeap` that exposes
paged_gc's own cycle index?  If so we should use that and drop our
own counter.  Otherwise no action — ours works fine and bumps in
exactly the spots we want.

## 9. Allocator failure semantics — already working, doc'd here for confirmation

`try_alloc_boxed_in` returns `Option<NonNull<u64>>` and we route to
`try_alloc_large` for runs greater than `PAGE_SIZE_CELLS`.  Both
return `None` on failure.  We surface this to Forth as throw `-2059`
(GC out of memory).

**Ask:** Confirm the failure modes that produce `None`:

- Reservation exhausted (no more pages in the address-space range)
- `n_cells > MAX_LENGTH` (24-bit length field would overflow)

Anything else?  In particular, can `try_alloc_large` fail because
the contiguous-page search can't find a run of N consecutive free
pages even though total free pages are sufficient (fragmentation
failure)?  If so, should the binding force a major-GC and retry, or
is the page allocator's own free-list management resilient to that?

## 10. Move-event hook — debug-mode nicety

For tracing / debugging, it would be useful to register a callback
that fires whenever an object moves: `(old_addr, new_addr,
type_byte) -> ()`.  In WF64's REPL we could turn this on with `(trace
gc)` and emit a line per object move.  Inside paged_gc this
information is already known at the evac sites — exposing it as an
optional `FnMut` callback would cost nothing when disabled.

Low priority — nice-to-have for production debugging once the SIMD
DSLs (V4/V5) start producing harder-to-trace allocation patterns.

---

## Summary

The minimum we need before V2 stage C lands cleanly:

1. **Static-region card table convenience (#1)** — or a worked
   example showing the existing API.
2. **Confirmation of write-barrier contract (#2)** — when to call
   `mark_card_at`, how cheap it is.
3. **Trigger setter (#4)** — so we can tune for small-object
   workloads later without a paged_gc fork.

Nice-to-have, in priority order:

4. **Scoped pin API (#3)** — before strings ship.
5. **Heap stats accessor (#5)** — improves test fidelity.
6. **Major-trigger pattern (#7)** — affects auto-GC heuristics.

Everything else (cycle counter alignment, heap-clear, allocator
failure semantics, move-event hook) is documentation/feature
convenience and can land whenever.

— WF64 Forth side, mid-V2
