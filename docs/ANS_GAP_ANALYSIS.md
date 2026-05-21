# WF64 ANS Forth Gap Analysis

**Date:** 2026-05-20  
**Current milestone:** M4 complete, M5 next  
**Definitive source of truth:** `src/lib.rs` PRIMITIVES table + `lib/core.f`

---

## Section 1: What's Done

All entries confirmed present in PRIMITIVES or core.f.

### 1.1 Stack Manipulation

| Word | Source |
|------|--------|
| `dup` `swap` `drop` `rot` `over` | PRIMITIVES |
| `-rot` `?dup` `nip` `tuck` `pick` | PRIMITIVES |
| `depth` `sp@` `sp!` | PRIMITIVES |
| `2dup` `2drop` `2swap` `2over` `2rot` `2nip` | PRIMITIVES |

### 1.2 Return Stack

| Word | Source |
|------|--------|
| `>r` `r>` `r@` `rdrop` | PRIMITIVES |
| `2>r` `2r>` `2r@` `2rdrop` | PRIMITIVES |
| `dup>r` `n>r` `nr>` | PRIMITIVES (non-ANS) |
| `rp@` `rp!` | PRIMITIVES |
| `i` `j` | PRIMITIVES |
| `unloop` | PRIMITIVES |

### 1.3 Memory

| Word | Source |
|------|--------|
| `@` `!` `c@` `c!` `2@` `2!` | PRIMITIVES |
| `here` `allot` | PRIMITIVES |
| `cells` `cell+` `chars` `char+` `aligned` | PRIMITIVES |
| `fill` `cmove` `cmove>` `move` `count` | PRIMITIVES |
| `+!` `c+!` | PRIMITIVES |
| `c,` `,` `2,` `align` | core.f |

### 1.4 Arithmetic

| Word | Source |
|------|--------|
| `+` `-` `*` `/` `mod` `/mod` | PRIMITIVES |
| `negate` `abs` | PRIMITIVES |
| `1+` `1-` `2+` `2-` `2*` `2/` | PRIMITIVES |
| `um*` `m*` `um/mod` `sm/rem` `fm/mod` | PRIMITIVES |
| `*/` `*/mod` | PRIMITIVES |
| `s>d` | PRIMITIVES |
| `d+` `d-` `dnegate` `dabs` `d2*` `d2/` | PRIMITIVES |

### 1.5 Logic and Comparison

| Word | Source |
|------|--------|
| `and` `or` `xor` `invert` | PRIMITIVES |
| `lshift` `rshift` | PRIMITIVES |
| `0=` `0<>` `0<` `0>` | PRIMITIVES |
| `=` `<>` `<` `>` `<=` `>=` | PRIMITIVES |
| `u<` `u>` `min` `max` `within` | PRIMITIVES |
| `d=` `d0<` `d0=` `d<` `du<` `d>` `d<>` | PRIMITIVES |
| `true` `false` | core.f |

### 1.6 Number I/O

| Word | Source |
|------|--------|
| `.` `.s` | PRIMITIVES (`.` via `rt_print_int`) |
| `>number` `digit` | PRIMITIVES |
| `base` `decimal` `hex` | PRIMITIVES |
| `u.` | core.f |
| `<#` `#` `#s` `#>` `hold` `holds` `sign` | core.f |
| `ud/mod` | core.f |
| `hld` (variable) | core.f |

### 1.7 String Operations

| Word | Source |
|------|--------|
| `/string` `search` `compare` `bounds` | PRIMITIVES |
| `skip` `-skip` `scan` `-scan` | PRIMITIVES |
| `place` `+place` `c+place` | PRIMITIVES |
| `count` `zcount` | PRIMITIVES |
| `upper` `lower` `uppercase` `lowercase` `upc` `tr` | PRIMITIVES |

### 1.8 Control Flow (compile-time words)

| Word | Source | Notes |
|------|--------|-------|
| `if` `then` `else` | PRIMITIVES (IMMEDIATE) | M5 target |
| `begin` `while` `again` `until` `repeat` | PRIMITIVES (IMMEDIATE) | M5 target |
| `do` `?do` `loop` `+loop` `-loop` | PRIMITIVES (IMMEDIATE) | M5 target |
| `leave` `?leave` `unloop` | PRIMITIVES | |
| `recurse` `ahead` | PRIMITIVES (IMMEDIATE) | |
| `bra` `?bra` `-?bra` `bra-?do` | PRIMITIVES (runtime jump) | |
| `_loop` `_+loop` `_-loop` | PRIMITIVES (runtime loop step) | |
| `mark>` `<resolve` `>resolve` `?pairs` | PRIMITIVES (compile helpers) | |
| `do-part1` `do-part2` | PRIMITIVES (runtime) | |

### 1.9 Compiler / Defining Words

| Word | Source |
|------|--------|
| `:` `;` `exit` `create` `does>` | PRIMITIVES |
| `[` `]` `immediate` | PRIMITIVES |
| `'` `[']` `literal` `postpone` | PRIMITIVES |
| `s"` `."` `abort"` | PRIMITIVES |
| `variable` `2variable` `constant` `2constant` | core.f |
| `fvariable` `f,` | core.f |

### 1.10 Parsing and Input

| Word | Source |
|------|--------|
| `parse-name` `parse` `word` | PRIMITIVES |
| `source` `source-id` `refill` `>in` | PRIMITIVES |
| `state` | PRIMITIVES |
| `evaluate` | PRIMITIVES |
| `accept` | PRIMITIVES |
| `pad` | PRIMITIVES |
| `number?` | PRIMITIVES |
| `bl` `space` `spaces` | core.f |

### 1.11 Dictionary / Wordlist / Vocabulary

| Word | Source |
|------|--------|
| `find-name` `name>interpret` `name>compile` | PRIMITIVES |
| `>body` `latestxt` `>name` `>ct` `>comp` | PRIMITIVES |
| `forth-wordlist` `wordlist` | PRIMITIVES |
| `get-current` `set-current` `definitions` | PRIMITIVES |
| `get-order` `set-order` | PRIMITIVES |
| `only` `also` `previous` `forth` | PRIMITIVES |
| `search-wordlist` | PRIMITIVES |

### 1.12 Execution and Exception

| Word | Source |
|------|--------|
| `execute` `perform` | PRIMITIVES |
| `catch` `throw` `abort` | PRIMITIVES |
| `quit` | PRIMITIVES |
| `bye` | PRIMITIVES |

### 1.13 I/O

| Word | Source |
|------|--------|
| `emit` `type` `key` `key?` `cr` | PRIMITIVES |

### 1.14 Floating Point

| Word | Source |
|------|--------|
| `f+` `f-` `f*` `f/` `fnegate` | PRIMITIVES |
| `f0=` `f0<` `f<` | PRIMITIVES |
| `f@` `f!` `fdup` `fdrop` `fswap` `fover` `fdepth` | PRIMITIVES |
| `d>f` `f>d` `float+` `floats` `falign` `faligned` | PRIMITIVES |
| `fliteral` | PRIMITIVES |
| `fvariable` | core.f |

### 1.15 `environment?`

Present as a stub in core.f: always returns `false`. Sufficient for M7.

---

## Section 2: Control Flow Gaps (M5)

The compile-time immediate words (`if`, `then`, `else`, `begin`, `while`, `until`, `repeat`, `do`, `?do`, `loop`, `+loop`, `leave`, `recurse`) are all already registered in PRIMITIVES with flag `1` (IMMEDIATE) and have corresponding kernel symbols. The runtime jump primitives (`bra`, `?bra`, `-?bra`, `bra-?do`, `_loop`, `_+loop`, `_-loop`) and compile-time helpers (`mark>`, `<resolve`, `>resolve`, `?pairs`, `do-part1`, `do-part2`) are also present.

**What M5 requires is not new words, but verification that the existing implementations are correct and tested.** The MILESTONES.md milestone says "done" when a countdown loop runs — meaning these need integration tests, not new primitives.

Specific things to verify and test:

| Mechanism | What to check | Implementation path |
|-----------|--------------|-------------------|
| `IF`/`THEN` | `if_word` emits `?bra`, `then_word` patches the forward ref | Already kernel |
| `IF`/`ELSE`/`THEN` | `else_word` emits `bra`, patches the `if` ref, marks a new forward ref | Already kernel |
| `BEGIN`/`UNTIL` | `begin_word` marks, `until_word` emits `?bra` backward | Already kernel |
| `BEGIN`/`WHILE`/`REPEAT` | `begin` marks, `while` emits `?bra` forward, `repeat` emits `bra` back and patches | Already kernel |
| `BEGIN`/`AGAIN` | `begin` marks, `again` emits unconditional `bra` back | Already kernel |
| `DO`/`LOOP` | `do_word` → `do_part1`/`do_part2` inline, `loop_control_word` patches | Already kernel |
| `?DO`/`LOOP` | `qdo_control_word` → `bra_qdo_word` with optional skip | Already kernel |
| `DO`/`+LOOP` | Same as DO/LOOP but `plus_loop_control_word` uses `_+loop` | Already kernel |
| `LEAVE` | `leave_word` emits `bra` to end of loop, patched by LOOP | Already kernel |
| `UNLOOP` | `unloop_word` removes DO frame from return stack | Already kernel |
| `RECURSE` | `recurse_word` emits call to current definition's xt | Already kernel |
| `I`, `J` | Runtime copies of loop index from return stack | Already kernel |
| `?pairs` mismatch check | Structural balance checking during compilation | Already kernel |

**M5 action:** Write eval tests for each control structure combination. No new kernel code expected; the gap is test coverage.

---

## Section 3: Missing CORE Wordset

Cross-referenced against ANS Forth (X3.215-1994) and Forth 2012 CORE wordset.

### 3.1 Present (no gap)

`!` `"` `#` `#>` `#s` `'` `(` `)` `*` `*/` `*/mod` `+` `+!` `+loop` `+loop` `,` `-` `.` `."` `/` `/mod` `0<` `0=` `1+` `1-` `2!` `2*` `2/` `2@` `2drop` `2dup` `2over` `2swap` `<` `<#` `=` `>` `>body` `>in` `>number` `>r` `?do` `@` `abort` `abort"` `abs` `accept` `align` `aligned` `allot` `and` `base` `begin` `bl` `c!` `c,` `c@` `cell+` `cells` `char+` `chars` `constant` `count` `create` `decimal` `depth` `do` `does>` `drop` `dup` `else` `emit` `evaluate` `execute` `exit` `fill` `find` `fm/mod` `here` `hold` `i` `if` `immediate` `invert` `j` `key` `leave` `literal` `loop` `lshift` `m*` `max` `min` `mod` `move` `negate` `or` `over` `postpone` `quit` `r>` `r@` `recurse` `refill` `repeat` `rot` `rshift` `s"` `s>d` `sign` `sm/rem` `source` `space` `spaces` `state` `swap` `then` `type` `u<` `um*` `um/mod` `unloop` `until` `variable` `while` `word` `xor` `[` `[']` `[char]` `]`

**Note on `find`:** ANS uses `FIND` with `( c-addr -- c-addr 0 | xt 1 | xt -1 )` (counted string). WF64 has `find-name` with a `( c-addr u -- nt -1 | c-addr u 0 )` interface (Forth 2012 style). A thin `FIND` wrapper is needed.

**Note on `[char]`:** Not in PRIMITIVES. Needs source-defined implementation.

### 3.2 Missing CORE Words

| Word | Stack effect | Description | Implementation | Difficulty |
|------|-------------|-------------|----------------|------------|
| `[char]` | `( "c" -- char )` | Compile-time char literal | source-defined Forth, uses `parse-name` + `char` (see below) | easy |
| `char` | `( "c" -- n )` | Parse next token, return ASCII of first char | source-defined Forth: `parse-name drop c@` | easy |
| `find` | `( c-addr -- c-addr 0 \| xt 1 \| xt -1 )` | ANS counted-string dict search | source-defined wrapper around `find-name` + `count` | easy |
| `2constant` | Already in core.f | — | done | — |
| `2variable` | Already in core.f | — | done | — |
| `>` | Already present | — | done | — |
| `u>` | Already present | — | done | — |
| `0>` | Already present | — | done | — |
| `key?` | Already present | — | done | — |
| `source-id` | Already present | — | done | — |
| `parse` | Already present | — | done | — |
| `false` | Already in core.f | — | done | — |
| `true` | Already in core.f | — | done | — |
| `nip` | Already present | — | done | — |
| `tuck` | Already present | — | done | — |
| `?dup` | Already present | — | done | — |
| `.r` | `( n width -- )` | Print n right-aligned in field of width | source-defined | easy |
| `u.r` | `( u width -- )` | Print u right-aligned in field | source-defined | easy |
| `d.` | `( d -- )` | Print double-cell signed | source-defined using `<#` | easy |
| `d.r` | `( d width -- )` | Print double right-aligned | source-defined | easy |
| `ud.` | `( ud -- )` | Print unsigned double | source-defined | easy |
| `hex` | Already present | — | done | — |
| `octal` | Already present (non-ANS bonus) | — | done | — |
| `number` | `( c-addr u -- n \| d )` | Parse number (for compatibility) | non-ANS, `number?` covers this | — |
| `?` | `( a-addr -- )` | Fetch and print | source-defined: `@ .` | trivial |
| `dump` | `( addr u -- )` | Hex dump memory | source-defined | medium |
| `erase` | `( addr u -- )` | Zero memory region | source-defined: `0 fill` | trivial |
| `unused` | `( -- u )` | Cells left in dictionary | source-defined: dict-end - here | easy |

**Summary:** The CORE wordset is largely complete. Missing words are small source-defined items, not kernel primitives.

---

## Section 4: Missing CORE EXT Wordset

ANS Forth CORE EXT adds words beyond the base CORE set.

### 4.1 Present or Effectively Present

| Word | Status |
|------|--------|
| `0<>` | present |
| `0>` | present |
| `2>r` `2r>` `2r@` | present |
| `<>` | present |
| `?do` `+loop` | present |
| `again` | present |
| `c"` | missing — see below |
| `erase` | trivial source-def |
| `false` `true` | core.f |
| `hex` | present |
| `nip` `tuck` | present |
| `pick` `roll` | `pick` present; `roll` missing |
| `refill` | present |
| `source-id` | present |
| `to` | missing — see below |
| `value` | missing — see below |
| `within` | present |
| `\` | missing — see below |
| `.(` | missing — see below |
| `:noname` | missing — see below |
| `parse-name` | present (Forth 2012) |
| `holds` | core.f |
| `compile,` | present as `compile_comma` helper |
| `action-of` | missing |
| `defer` | missing |
| `defer!` `defer@` | missing |
| `buffer:` | missing (trivial) |
| `is` | missing (alias for `to` on defers) |

### 4.2 Missing CORE EXT Words

| Word | Stack | Description | Implementation | Difficulty |
|------|-------|-------------|----------------|------------|
| `\` | `( "line" -- )` | Line comment | kernel MASM (IMMEDIATE, skip to EOL) or source-defined via `parse` dropping result | easy |
| `(` | `( "ccc<paren>" -- )` | Comment (paren) | source-defined or kernel; skip to `)` using `parse` | easy |
| `.(` | `( "ccc<paren>" -- )` | Print comment | source-defined: `41 parse type` | easy |
| `c"` | `( "ccc" -- c-addr )` | Counted string literal (compile only) | kernel MASM primitive (like `s"` but counted) | medium |
| `roll` | `( xu...x0 u -- xu-1...x0 xu )` | Rotate u-th stack item to top | kernel MASM (can't be source-defined safely) | medium |
| `:noname` | `( -- xt )` | Anonymous colon definition | kernel MASM, starts colon without `create` | medium |
| `to` | `( x "<spaces>name" -- )` | Store to VALUE | kernel (IMMEDIATE + parse + state-smart) | medium |
| `value` | `( x "<spaces>name" -- )` | Named constant with `to` setter | source-defined + `to` | medium |
| `defer` | `( "<spaces>name" -- )` | Deferred word (late binding) | source-defined using `create` + vectored execute | medium |
| `defer!` | `( xt2 xt1 -- )` | Set defer target | source-defined | easy |
| `defer@` | `( xt1 -- xt2 )` | Get defer target | source-defined | easy |
| `action-of` | `( "<spaces>name" -- xt )` | Get deferred xt | source-defined (state-smart) | medium |
| `is` | `( xt "<spaces>name" -- )` | Set defer (alias of defer!) | source-defined | easy |
| `buffer:` | `( u "<spaces>name" -- )` | Named buffer allocation | source-defined: `create allot` | trivial |
| `erase` | `( addr u -- )` | Zero u bytes at addr | source-defined: `0 fill` | trivial |
| `unused` | `( -- u )` | Free dictionary space | source-defined | easy |
| `?` | `( a-addr -- )` | Print contents of address | source-defined: `@ .` | trivial |
| `dump` | `( addr u -- )` | Hex dump | source-defined | medium |
| `u>` | Already present | — | done | — |
| `marker` | `( "<spaces>name" -- )` | Create a forget-marker | kernel MASM (relies on FORGET infrastructure) | hard |
| `compile,` | `( xt -- )` | Append xt to definition | present as `compile_comma` internal; needs dict name | easy (rename) |

---

## Section 5: Other Wordsets

### 5.1 EXCEPTION Wordset

| Word | Status | Notes |
|------|--------|-------|
| `catch` | **present** | kernel MASM |
| `throw` | **present** | kernel MASM |
| `abort` | **present** | kernel MASM |
| `abort"` | **present** | kernel MASM |

**Gap:** None. EXCEPTION is complete.

### 5.2 FILE Wordset (M6 target)

All file words are missing. M6 is the planned milestone.

| Word | Status |
|------|--------|
| `open-file` | missing |
| `create-file` | missing |
| `close-file` | missing |
| `read-file` | missing |
| `read-line` | missing |
| `write-file` | missing |
| `write-line` | missing |
| `file-position` | missing |
| `reposition-file` | missing |
| `resize-file` | missing |
| `file-size` | missing |
| `file-status` | missing |
| `delete-file` | missing |
| `rename-file` | missing |
| `include-file` | missing |
| `included` | missing |
| `include` | missing |
| `require` | missing |
| `required` | missing |
| `source-id` when file | partial (present, returns 0 for console) |
| `flush-file` | missing |

**Implementation path:** Win32 `CreateFileW`, `ReadFile`, `WriteFile`, `CloseHandle`. Source IDs: −1=string eval, 0=user input, positive=file handle. Source-stack for nested `include` needed (save/restore `>in`, `source`, `source-id`).

### 5.3 DOUBLE Wordset

Most double-cell arithmetic is present as primitives. What's missing:

| Word | Status | Notes |
|------|--------|-------|
| `2constant` | **present** (core.f) | |
| `2variable` | **present** (core.f) | |
| `2literal` | missing | compile-time double literal (push two cells) |
| `2value` | missing | depends on `value`/`to` infrastructure |
| `d+` `d-` `dnegate` `dabs` `d*` | **present** | `d*` not in PRIMITIVES but can be source-defined |
| `d<` `d=` `du<` `d0<` `d0=` | **present** | |
| `d.` `d.r` | missing | source-defined using `<#` |
| `du.` `ud.` | missing | source-defined |
| `dmax` `dmin` | missing | source-defined |

**Gap summary:** small — `2literal`, `d.`, `d.r`, `dmax`, `dmin`, `2value`.

### 5.4 STRING Wordset

| Word | Status | Notes |
|------|--------|-------|
| `cmove` `cmove>` | **present** | |
| `move` | **present** | |
| `search` | **present** | |
| `compare` | **present** | |
| `-trailing` | missing | Remove trailing spaces from string |
| `sliteral` | present as kernel helper `sliteral` | needs dict entry |
| `char` | missing | see CORE |
| `upc` `lower` `upper` | **present** (non-ANS extensions) | |

**Gap:** `-trailing` is the only ANS STRING word absent.

### 5.5 MEMORY Wordset

| Word | Status | Notes |
|------|--------|-------|
| `allocate` | missing | Heap allocate (malloc-like) |
| `free` | missing | |
| `resize` | missing | |

**Implementation path:** Win32 `HeapAlloc`/`HeapFree`/`HeapReAlloc` via `GetProcessHeap`. All three are medium-difficulty kernel MASM calls.

### 5.6 TOOLS Wordset

| Word | Status | Notes |
|------|--------|-------|
| `.s` | **present** | |
| `?` | missing | `@ .` — trivial source-def |
| `dump` | missing | hex dump — source-defined, medium |
| `words` | missing | list dictionary — source-defined, medium |
| `see` | missing | decompile word — hard |
| `n>r` `nr>` | **present** (non-ANS) | |

### 5.7 FLOAT and FLOAT EXT Wordsets

The ANS FLOAT wordset is substantially present (core arithmetic, stack ops, comparison, conversions). Gaps:

| Word | Status |
|------|--------|
| `fabs` | missing — source-defined: `fdup f0< if fnegate then` |
| `fmax` `fmin` | missing — source-defined |
| `floor` `fround` `ftruncate` | missing — kernel MASM (SSE2 `roundsd`) |
| `fsqrt` `fexp` `fln` `flog` `fasin` `facos` `fatan` `fatan2` `fsin` `fcos` `ftan` `fsinh` `fcosh` `ftanh` `fasinh` `facosh` `fatanh` | missing — math lib calls |
| `f.` `fe.` `fs.` | missing — pictured float output |
| `precision` `set-precision` | missing — float print precision |
| `represent` | missing — float→string |
| `f~` | missing — approximate equality |
| `float` | missing — parse float from string |
| `>float` | missing — convert string to float |

For M7, only the words exercised by the ANS core test suite matter. Float words are in FLOAT/FLOAT EXT, not CORE — the core test suite will not require them.

---

## Section 6: Prioritised Implementation Order

### Sprint 5 — M5: Control Flow Integration Tests

**Goal:** Pass eval tests for all control-flow patterns; close M5.

**Work:**
- Write eval tests: `if`/`then`/`else`, `begin`/`until`, `begin`/`while`/`repeat`, `do`/`loop`, `do`/`+loop`, `?do`/`loop`, `leave`, nested loops, `recurse`
- Write direct tests for `i`, `j`, `unloop`
- Debug any issues found

**Done when:** All control flow eval tests pass. The MILESTONES.md countdown demo runs.

---

### Sprint 6 — Comments and Parse Completions

**Goal:** Comments (`\` and `(`) work; `char`/`[char]` work; `find` wrapper; `?` and `erase`.

**Words:**
- `\` — line comment (IMMEDIATE; source-defined using `parse` or `source` drop `>in !`)
- `(` — paren comment (IMMEDIATE; source-defined: `41 parse 2drop`)
- `.(` — print comment: `41 parse type`
- `char` — `parse-name drop c@`
- `[char]` — IMMEDIATE: `char postpone literal`
- `find` — ANS wrapper: `count find-name ...` (careful: `find` takes counted string, returns xt + 1/-1 or addr + 0)
- `?` — `@ .`
- `erase` — `0 fill`
- `buffer:` — `create allot`

All source-definable in core.f. No kernel changes.

**Done when:** `( this is a comment )` and `\ this too` parse silently; `char A` leaves 65; `find` returns correct results.

---

### Sprint 7 — Number Output Completions

**Goal:** Complete the numeric output vocabulary that M7 test suite exercises.

**Words:**
- `.r` — `( n w -- )` right-justified in field width
- `u.r` — `( u w -- )`
- `d.` — `( d -- )`
- `d.r` — `( d w -- )`
- `ud.` — `( ud -- )`

All source-definable using `<#`/`#`/`#s`/`sign`/`#>` which are already in core.f.

**Done when:** `3 5 .r` prints `    3`; `d.` prints a signed double.

---

### Sprint 8 — `:noname`, `VALUE`, `DEFER`, `TO`

**Goal:** Anonymous definitions, `value`/`to` pattern, deferred words.

**Words:**
- `:noname` — kernel MASM: like `:` but without `parse-name`/`create`; pushes xt
- `value` — source-defined: `create , does> @` + install `to`-aware compile action
- `to` — kernel MASM IMMEDIATE: state-smart; in interpret mode writes to body; in compile mode emits store
- `defer` — source-defined: `create ['] abort , does> @xt execute`
- `defer!` — `>body !`
- `defer@` — `>body @`
- `action-of` — IMMEDIATE: state-smart, returns xt of deferred word
- `is` — alias for `to` applied to defers

**Done when:** `: test-val 42 value x  x . ` prints 42; `43 to x  x .` prints 43.

---

### Sprint 9 — COMPILE, / ROLL / COMPILE-ONLY Utilities

**Goal:** Fill remaining structural gaps: `roll`, `compile,`, `-trailing`, `2literal`, `dmax`, `dmin`.

**Words:**
- `roll` — kernel MASM (rotate stack item): iterative swap loop is risky; direct asm safer
- `compile,` — expose `compile_comma` kernel helper in dictionary as `compile,`
- `-trailing` — source-defined: scan backwards dropping spaces
- `2literal` — IMMEDIATE: compiles two literals; source-defined using `literal`
- `dmax` `dmin` — source-defined using `d<`
- `fabs` `fmax` `fmin` — source-defined from float ops
- `unused` — source-defined: dict-end address minus `here`

**Done when:** `roll` passes tests; `compile,` is callable from Forth.

---

### Sprint 10 — M6: File I/O

**Goal:** Close M6 — `include` works, source stack correct for nested files.

**Words (all kernel MASM unless noted):**
- `open-file` — Win32 `CreateFileW`
- `create-file` — Win32 `CreateFileW` with create disposition
- `close-file` — `CloseHandle`
- `read-file` — `ReadFile`
- `read-line` — `ReadFile` + scan for newline
- `write-file` — `WriteFile`
- `write-line` — `write-file` + CR/LF emit
- `file-position` `reposition-file` — `SetFilePointerEx`
- `file-size` — `GetFileSizeEx`
- `flush-file` — `FlushFileBuffers`
- `file-status` — `GetFileAttributesW`
- `include-file` — kernel: push source frame, feed file to interpreter, pop
- `included` `include` — source-defined wrappers
- `require` `required` — source-defined: track included files, skip if already loaded

**Infrastructure needed:**
- Source frame stack (save/restore `>in`, input buffer pointer, `source-id`)
- Unicode filename conversion helper (UTF-8 Forth string → UTF-16 for Win32)

**Done when:** `include lib/test.f` loads and executes a file.

---

### Sprint 11 — MEMORY Wordset

**Goal:** Heap allocate/free available to Forth programs.

**Words:**
- `allocate` — `GetProcessHeap` + `HeapAlloc`; returns `( u -- a-addr ior )`
- `free` — `HeapFree`; returns `( a-addr -- ior )`
- `resize` — `HeapReAlloc`; returns `( a-addr1 u -- a-addr2 ior )`

All kernel MASM. IOR = 0 on success, Win32 error code on failure.

**Done when:** `256 allocate throw` allocates, `free` frees without crash.

---

### Sprint 12 — ANS Core Test Suite (M7)

**Goal:** Run the John Hayes ANS Forth core test suite and pass it.

**Work:**
- Port/obtain the ANS core test suite (tester.fr + core.fr) — these are standard files from https://forth-standard.org/standard/testsuite
- Adapt `load` mechanism to feed them through `include`
- Fix any failures reported

**Common failure categories to anticipate:**
- `FIND` semantics (returns `xt 1` for IMMEDIATE, `xt -1` for normal) — needs validation
- Number parsing edge cases (negative numbers, double-cell `100.`)
- `ENVIRONMENT?` — stub returns `false` which is acceptable for most queries
- `CHAR` / `[CHAR]` — must be in place (Sprint 6)
- Return stack discipline in `DO`/`LOOP` — needs `UNLOOP` before `EXIT`

**Done when:** Test suite reports 0 failures on CORE wordset.

---

## Section 7: Infrastructure Gaps

### 7.1 Number Parsing

**State:** `>number` and `number?` are present. The interpreter calls `number?` for token dispatch.

**Gap:** Double-number syntax (`100.` in ANS Forth produces a double-cell integer). WF64's `number?` may or may not handle the trailing-dot convention. This needs verification before M7.

**Gap:** Negative numbers — ANS requires `-5` to parse as a signed integer via `negate`. Check that `number?` handles the leading minus.

**Gap:** `'c'` character literal syntax is in some Forth dialects but not ANS CORE; `char` / `[char]` cover the ANS way.

### 7.2 Pictured Numeric Output

**State:** `<#` `#` `#s` `#>` `hold` `holds` `sign` are all in core.f. This is complete.

**Remaining:** `.r`, `u.r`, `d.`, `d.r` wrappers (Sprint 7).

### 7.3 Exception Frame

**State:** `catch`/`throw` are present in kernel MASM with a handler chain in the user area (`USER_HANDLER_VAR`). This is complete.

**Gap:** Test the `catch`/`throw`/`abort` round-trip behaviour under nested calls, especially through `do`/`loop` (needs `unloop` before re-`throw`). Needs integration tests.

### 7.4 `FIND` ANS Compatibility

WF64 has `find-name` (Forth 2012: takes `c-addr u`, returns name token). ANS Forth's `FIND` takes a counted string `( c-addr -- c-addr 0 | xt 1 | xt -1 )`. A wrapper is needed (Sprint 6):

```forth
: find ( c-addr -- c-addr 0 | xt 1 | xt -1 )
    count find-name
    dup 0= if drop 0 exit then
    \ nt on stack; get xt and IMMEDIATE flag
    dup name>interpret swap tfa@
    1 and if 1 else -1 then ;
```

### 7.5 Source Stack for Nested INCLUDE

**State:** `source`, `source-id`, `>in` are present. `evaluate` works for string eval.

**Gap:** Multi-level `include` requires saving and restoring the full source context (input buffer address, length, `>in`, `source-id`). This is the main M6 infrastructure task. Needs either a Forth-level source-stack array or kernel-managed frames.

### 7.6 Wordlist / Vocabulary Infrastructure

**State:** `forth-wordlist`, `wordlist`, `get-order`, `set-order`, `get-current`, `set-current`, `only`, `also`, `previous`, `forth`, `definitions`, `search-wordlist` are all present.

**Gap:** `marker` (CORE EXT) needs `forget`-style rollback. WF64 has `forget_last` and a forget-fence mechanism. A clean `marker` implementation needs to snapshot HERE + LATEST and create a word that restores them when executed. This is medium difficulty.

### 7.7 `compile,` Dictionary Exposure

`compile_comma` exists as a kernel symbol and is used internally. It needs a dictionary entry as `compile,` (Forth 2012 / CORE EXT) so Forth code can call it directly. This is a one-line addition to `core.f` or the PRIMITIVES table.

### 7.8 State-Smart Words (`TO`, `ACTION-OF`)

`TO` and `ACTION-OF` must be state-smart: behave differently in interpret vs. compile mode. This requires `state` checking inside an IMMEDIATE word. Standard pattern using `state @` + conditional compilation. The kernel MASM approach (check `user_STATE` directly) is cleaner for performance. Sprint 8.

### 7.9 ANS Test Harness Infrastructure

The standard ANS test files (`tester.fr`, `core.fr`) use:
- `\` comments — Sprint 6
- `(` comments — Sprint 6
- `include` / file loading — Sprint 10 (M6)
- `value` — Sprint 8
- `:noname` — Sprint 8
- String comparison for test reporting

All dependencies have sprint assignments. The test harness itself can be loaded once M6 is complete.

### 7.10 `DOT` / Numeric Output Correctness

**State:** `.` currently calls `rt_print_int` (Rust runtime, decimal only). This works for decimal output.

**Gap:** `.` must respect `BASE`. In hex mode, `255 .` should print `FF ` not `255 `. The M7 test suite will test this. Solution: replace the `rt_print_int` shortcut with a call to `u.` (which uses `<#`/`BASE`) in core.f, then make the kernel's `dot` call that. Or patch `dot` in core.f to redefine `.` as `s>d u.d` (which uses base correctly).

Proposed fix (core.f, after all picnum words defined):
```forth
: . ( n -- ) s>d swap over dabs <# #s rot sign #> type space ;
```
This replaces the `rt_print_int` shortcut at the Forth level. Medium — needs careful testing.

---

## Quick Reference: Sprint Summary

| Sprint | Milestone | Primary Goal | ~Words |
|--------|-----------|-------------|--------|
| 5 | M5 | Control flow tests | 0 new words; tests only |
| 6 | — | Comments, `char`, `find` wrapper | ~10 source-defined |
| 7 | — | Number output completions | ~5 source-defined |
| 8 | — | `:noname`, `value`/`to`, `defer` | ~8 (2 kernel, 6 source) |
| 9 | — | `roll`, `compile,`, `2literal`, misc | ~8 (1 kernel, 7 source) |
| 10 | M6 | File I/O | ~15 kernel, ~5 source |
| 11 | — | MEMORY wordset | 3 kernel |
| 12 | M7 | ANS core test suite pass | bug-fixes; no new words expected |
