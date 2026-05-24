# WF64 ANS Forth Gap Analysis

**Date:** 2026-05-22  
**Current status:** M5, M6, and M7 are complete; `cargo test` is green.  
**Definitive source of truth:** `src/lib.rs` `PRIMITIVES`, `lib/core.f`, and the active tests under `tests/`.

---

## 1. Summary

This file replaces the older M4-era analysis. That older snapshot is now badly stale: many words it listed as missing are implemented today either as kernel primitives or as source-defined words in `lib/core.f`.

The current picture is:

- The CORE wordset is largely in place.
- The practical CORE-EXT surface used by the current system and the ANS core tests is also largely in place.
- Control flow, source loading, and the ANS core tests are all working and covered.
- The main remaining gaps are no longer in the basic core language; they are in the broader FILE, MEMORY, and FLOAT extensions, plus a small number of genuinely absent convenience words.

---

## 2. Confirmed Present Now

These were notable historical gaps but are implemented in the current tree.

### 2.1 Core / Core-Ext Words Now Present

- `char`
- `[char]`
- `find` (ANS counted-string wrapper over `find-name`)
- `?`
- `erase`
- `unused`
- `.r`
- `u.r`
- `d.`
- `d.r`
- `ud.`
- `du.`
- `buffer:`
- `value`
- `to`
- `+to`
- `defer`
- `defer!`
- `defer@`
- `action-of`
- `is`
- `2literal`
- `:noname`
- `compile,`
- `c"`
- `roll`
- `included`
- `include`
- `save-input`
- `restore-input`
- `name>string`
- `.(`
- `marker`
- `[defined]`
- `[undefined]`
- `[if]`
- `[else]`
- `[then]`

### 2.2 Other Present Extensions

- `-trailing`
- `dmax`
- `dmin`
- `fabs`
- `fmax`
- `fmin`
- `words`
- `dump`
- string substitution helpers (`replaces`, `substitute`)
- source-defined CASE family (`case`, `of`, `endof`, `endcase`)

### 2.3 Comment Handling

Comment behavior is present, but implemented lexically in `parse_name` rather than by dictionary entries:

- `(` begins a comment only when it is the standalone token `(`
- `\` begins a line comment only when it is the standalone token `\`

That means comment syntax works correctly in source input even though the parser, not a normal word lookup, is what enforces it.

---

## 3. What Is Genuinely Still Missing

This section is intentionally narrower than the old document. It lists words and areas that still appear absent after checking the current primitive table and `lib/core.f`.

### 3.1 Small Language-Surface Gaps

These are the most obvious still-missing ANS-facing words.

| Word | Status | Notes |
|---|---|---|
| `2value` | missing | Not present; `to` remains single-cell oriented in the current implementation. |
| `environment?` | partial | Present only as a stub returning `false`. Good enough for current tests, not a real environment query database. |

### 3.2 FILE Wordset

WF64 now supports source loading via `included` / `include`, but that is not the same as implementing the ANS FILE wordset.

Still missing as user-visible words:

- `open-file`
- `create-file`
- `close-file`
- `read-file`
- `read-line`
- `write-file`
- `write-line`
- `flush-file`
- `file-position`
- `reposition-file`
- `resize-file`
- `file-size`
- `file-status`
- `delete-file`
- `rename-file`
- `include-file`
- `require`
- `required`

Current M6 implementation note:

- `include` / `included` are source-defined in `lib/core.f`
- file bytes are loaded by Rust runtime helpers (`rt_slurp_file`, `rt_slurp_len`, `rt_slurp_pop`)
- evaluation is routed back through `evaluate`

So: source loading exists, but the general FILE API does not.

### 3.3 MEMORY Wordset

Still missing:

- `allocate`
- `free`
- `resize`

These would need host heap bindings and a stable pointer/error contract.

### 3.4 FLOAT / FLOAT-EXT Gaps

Core float arithmetic is present, but many ANS float extras are still absent.

Most obvious missing groups:

- float formatting: `f.`, `fe.`, `fs.`
- precision control: `precision`, `set-precision`, `represent`
- parsing/conversion extras: `>float`, `float`
- approximate compare: `f~`
- rounding helpers beyond the current source-defined truncation path
- transcendental math (`fsin`, `fcos`, `ftan`, `fln`, `fexp`, `fsqrt`, etc.)

### 3.5 TOOLS Gaps

Present already:

- `.s`
- `?`
- `dump`
- `words`

Still clearly missing:

- `see`

---

## 4. Current Coverage Notes

### 4.1 Control Flow

This is no longer a gap. The control-flow surface is implemented and covered by harness tests, including:

- `if` / `else` / `then`
- `begin` / `until`
- `begin` / `while` / `repeat`
- `do` / `?do` / `loop` / `+loop` / `-loop`
- `leave` / `?leave`
- `recurse`
- `i` / `j`

### 4.2 ANS Core Tests

The repo now includes:

- `lib/tester.fs`
- `lib/ans_core_tests.fs`
- harness coverage that loads and runs those tests

So the earlier “M7 next” framing is obsolete.

---

## 5. Suggested Next Additions

If the goal is to keep closing real ANS gaps with good cost/benefit, the next sensible order is:

1. real `environment?`
   Reason: current stub is acceptable for tests but not a real implementation.

2. `2value`
   Reason: the value/to family is otherwise present; this is the most obvious remaining hole in that area.

3. FILE wordset subset
   Reason: `include` exists already; exposing actual file handles and read/write operations is the next coherent step.

4. MEMORY wordset (`allocate` / `free` / `resize`)
   Reason: small, well-bounded host interop task.

5. float text I/O (`f.` / `fe.` / `fs.`)
   Reason: current float arithmetic is strong, but human-facing float tooling is still thin.

---

## 6. Bottom Line

WF64 is no longer missing lots of core ANS words. The current gaps are concentrated in:

- `2value`
- real `environment?`
- the general FILE wordset
- the MEMORY wordset
- float formatting / FLOAT-EXT breadth
- `see`

Anything that still lists `c"`, `roll`, `to`, `value`, `defer`, `include`, `dump`, `words`, comments, or the basic numeric output family as missing should be treated as outdated.
