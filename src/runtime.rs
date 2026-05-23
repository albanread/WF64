//! Rust runtime functions the kernel calls via `@extern`.
//!
//! All I/O is routed through a thread-local `Session` (see `lib.rs`).
//! Tests use a session whose `input`/`output` are in-memory buffers;
//! the interactive REPL wrapper uses a session backed by stdin/stdout.
//!
//! Routing through a session-scoped buffer (instead of writing directly
//! to stdout) is what makes the test harness possible: each `#[test]`
//! owns its session, feeds input, reads output, and inspects the data
//! stack — no global state, no pipes, no temp files.

use std::cell::RefCell;
use std::io::Write;

fn normalize_float_token(text: &str) -> Option<String> {
    if text.is_empty() {
        return None;
    }
    if !text.bytes().any(|b| b == b'.' || b == b'e' || b == b'E') {
        return None;
    }

    let mut out = text.trim().to_string();
    if out.is_empty() {
        return None;
    }

    if let Some(pos) = out.find(['e', 'E']) {
        if pos + 1 == out.len() {
            out.push('0');
        } else {
            let bytes = out.as_bytes();
            if (bytes[pos + 1] == b'+' || bytes[pos + 1] == b'-') && pos + 2 == out.len() {
                out.push('0');
            }
        }
    }

    Some(out)
}

/// I/O backing for one session. Tests use the `Buffered` variant; the
/// interactive REPL uses `Live`.
pub enum Io {
    /// In-memory buffers — for tests. `input` is consumed by
    /// `rt_read_line`, `output` accumulates bytes written by emit/type.
    Buffered {
        input: Vec<u8>,
        in_cursor: usize,
        pending_key: Option<u8>,
        output: Vec<u8>,
    },
    /// Real stdin/stdout — for the interactive REPL.
    Live {
        pending_key: Option<u8>,
    },
}

impl Io {
    pub fn new_buffered() -> Self {
        Io::Buffered { input: Vec::new(), in_cursor: 0, pending_key: None, output: Vec::new() }
    }
}

#[cfg(windows)]
unsafe extern "C" {
    fn _kbhit() -> i32;
    fn _getwch() -> u16;
}

thread_local! {
    /// The currently-active session's I/O. `Wf64Session::enter` swaps
    /// itself in for the duration of an `eval`; outside of that it's
    /// `None` and runtime calls will panic.
    static CURRENT_IO: RefCell<Option<Io>> = const { RefCell::new(None) };
}

/// Install `io` as the current session's I/O, run `f`, swap out and
/// return both the function's result and the (possibly-mutated) Io.
/// Restores any previously-installed Io on the way out.
///
/// Not panic-safe: if `f` panics, `CURRENT_IO` is left holding the new
/// `Io` and the test thread dies. That's acceptable for harness code —
/// production paths shouldn't panic past this point.
pub fn with_io<R>(io: Io, f: impl FnOnce() -> R) -> (R, Io) {
    let prev = CURRENT_IO.with(|cell| cell.replace(Some(io)));
    let result = f();
    let io_after = CURRENT_IO
        .with(|cell| cell.replace(prev))
        .expect("CURRENT_IO must be Some inside with_io");
    (result, io_after)
}

/// Quick accessor used by runtime functions: panic if there's no
/// session bound (that would indicate a logic bug in the harness).
fn with_current_io<R>(f: impl FnOnce(&mut Io) -> R) -> R {
    CURRENT_IO.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let io = borrow
            .as_mut()
            .expect("WF64 runtime called outside of a Wf64Session::eval");
        f(io)
    })
}

/// Print a signed cell in decimal followed by a single space, no newline.
#[no_mangle]
pub extern "C" fn rt_print_int(n: u64) -> u64 {
    let s = n as i64;
    let bytes = format!("{s} ").into_bytes();
    write_bytes(&bytes);
    0
}

/// Print the live Forth stack without consuming it.
///
/// The kernel passes its internal TOS cache plus DSP/SP0 so we can
/// reconstruct the logical stack shape without forcing a restart or a
/// spill/reload cycle through forth_main.
#[no_mangle]
pub extern "C" fn rt_dot_s(tos: u64, dsp: u64, sp0: u64, rsp: u64) -> u64 {
    let depth = if dsp > sp0 {
        0usize
    } else {
        ((sp0 - dsp) / 8 + 1) as usize
    };

    if depth == 0 {
        write_bytes(format!("[empty sp={dsp:#x} rp={rsp:#x}]").as_bytes());
        return 0;
    }

    write_bytes(format!("[{depth} sp={dsp:#x} rp={rsp:#x}] ").as_bytes());
    write_bytes(format!("{} ", tos as i64).as_bytes());
    for index in 1..depth {
        let addr = dsp + (index as u64 - 1) * 8;
        let value = unsafe { (addr as *const i64).read_unaligned() };
        write_bytes(format!("{value} ").as_bytes());
    }
    0
}

/// Forth-tuned breakpoint dump.
///
/// Called by `brk` / `int3` before the INT 3 instruction so the human
/// sees a readable Forth state before the raw VEH register dump.
///
/// Arguments (Win64, 5-arg):
///   tos   — cached TOS register
///   dsp   — data stack pointer (points at NOS)
///   sp0   — initial DSP (base of data stack)
///   rsp   — Forth return stack pointer at the point of the breakpoint
///   up    — user area pointer (= rsp_top since region layout makes them equal)
///
/// # Safety
/// All pointers come from the live JIT session arena.
#[no_mangle]
pub extern "C" fn rt_forth_brk(tos: u64, dsp: u64, sp0: u64, rsp: u64, up: u64) -> u64 {
    let mut out = String::with_capacity(512);

    out.push_str("\n=== Forth Breakpoint ==================================================\n");

    // ── Data stack ──────────────────────────────────────────────────
    let depth = if dsp > sp0 {
        0usize
    } else {
        ((sp0 - dsp) / 8 + 1) as usize
    };
    out.push_str(&format!("Data stack [{depth}]:\n"));
    if depth == 0 {
        out.push_str("  (empty)\n");
    } else {
        out.push_str(&format!("  TOS: {:>20}  {:#018x}\n", tos as i64, tos));
        for i in 1..depth {
            let addr = dsp + (i as u64 - 1) * 8;
            let v = unsafe { (addr as *const u64).read_unaligned() };
            out.push_str(&format!("  {:>3}: {:>20}  {:#018x}\n", i, v as i64, v));
        }
    }

    // ── Return stack ────────────────────────────────────────────────
    // rsp_top == up (region layout: return stack grows from up downward).
    let rstack_depth = if rsp >= up { 0usize } else { ((up - rsp) / 8) as usize };
    let rstack_show  = rstack_depth.min(16);
    out.push_str(&format!("Return stack [{rstack_depth} cells, showing {rstack_show}]:\n"));
    for i in 0..rstack_show {
        let addr = rsp + i as u64 * 8;
        let v = unsafe { (addr as *const u64).read_unaligned() };
        out.push_str(&format!("  [{i}]: {v:#018x}\n"));
    }

    // ── Key user variables ───────────────────────────────────────────
    // Safety: up points into the live session user area.
    let uread = |off: u64| unsafe { *((up + off) as *const u64) };
    let base         = uread(0x00);
    let state        = uread(0x08);
    let latest       = uread(0x10);
    let here         = uread(0x18);
    let latestxt     = uread(0x78);
    let handler      = uread(0x80);
    let throw_code   = uread(0x88);
    let current      = uread(0x1500);
    let forth_wid    = uread(0x1508);
    let order_count  = uread(0x1510);
    out.push_str("User variables:\n");
    out.push_str(&format!("  BASE={base:<5}  STATE={state:<3}  HERE={here:#x}  LATEST={latest:#x}\n"));
    out.push_str(&format!("  LATESTXT={latestxt:#x}  HANDLER={handler:#x}  THROW={throw_code}\n"));
    out.push_str(&format!("  CURRENT={current:#x}  FORTH-WID={forth_wid:#x}  ORDER={order_count}\n"));
    let show_ctx = (order_count as usize).min(16);
    for i in 0..show_ctx {
        let wid = uread(0x1528 + i as u64 * 8);
        out.push_str(&format!("  CONTEXT[{i}]={wid:#x}\n"));
    }

    out.push_str("=======================================================================\n");
    write_bytes(out.as_bytes());
    0
}

/// Per-word trace hook, called from the interpreter before each word executes.
///
/// Arguments (Win64, 4-arg):
///   nt    — name token (pointer to counted string: length byte then chars)
///   tos   — current TOS
///   dsp   — current DSP (points at NOS)
///   sp0   — initial DSP (base of data stack)
///
/// # Safety
/// `nt` points into the live JIT dictionary arena.
#[no_mangle]
pub extern "C" fn rt_forth_trace(nt: u64, tos: u64, dsp: u64, sp0: u64) -> u64 {
    let name = unsafe {
        let len = *(nt as *const u8) as usize;
        let bytes = std::slice::from_raw_parts((nt + 1) as *const u8, len);
        std::str::from_utf8(bytes).unwrap_or("<?>")
    };

    let depth = if dsp > sp0 {
        0usize
    } else {
        ((sp0 - dsp) / 8 + 1) as usize
    };

    let mut out = format!("» {name:<16}  (");
    if depth == 0 {
        out.push_str(" empty");
    } else {
        out.push_str(&format!(" {}", tos as i64));
        for i in 1..depth {
            let addr = dsp + (i as u64 - 1) * 8;
            let v = unsafe { (addr as *const i64).read_unaligned() };
            out.push_str(&format!(" {v}"));
        }
    }
    out.push_str(" )\n");
    write_bytes(out.as_bytes());
    0
}

/// Write one byte to current output.
#[no_mangle]
pub extern "C" fn rt_emit(ch: u64) -> u64 {
    let byte = ch as u8;
    write_bytes(&[byte]);
    0
}

/// Write `len` bytes from `addr` to current output.
///
/// # Safety
/// The JITed `type` primitive guarantees `[addr, addr+len)` is readable.
#[no_mangle]
pub extern "C" fn rt_type(addr: u64, len: u64) -> u64 {
    if len == 0 {
        return 0;
    }
    let slice = unsafe { std::slice::from_raw_parts(addr as *const u8, len as usize) };
    write_bytes(slice);
    0
}

/// Cooperative-bye: this no longer terminates the process. The kernel's
/// `bye` primitive sets `user_BYE_REQ` directly and quit returns
/// cleanly. The interactive REPL wrapper turns that clean return into a
/// `process::exit` itself.
///
/// Kept exported because the win32 bindings list it; harmless no-op now.
#[no_mangle]
pub extern "C" fn rt_bye(_code: u64) -> u64 {
    0
}

/// Read one line of input into `buf` (at most `cap` bytes, terminator
/// not included).
///
/// Return value:
///   * `0..=cap` — number of bytes written (0 means empty line — *not*
///     end of input).
///   * `u64::MAX` (all 1s) — end of input. The kernel's `accept`
///     forwards this to `quit`, which treats it as an implicit `bye`.
///
/// This separation lets the REPL handle blank lines correctly while
/// still terminating cleanly on stdin EOF or end of a buffered input.
///
/// In `Buffered` mode reads from the session's in-memory input buffer.
/// In `Live` mode reads from stdin via `BufRead::read_line`, which
/// works equally on consoles and on redirected stdin.
///
/// # Safety
/// The kernel's `accept` guarantees `[buf, buf+cap)` is writable.
#[no_mangle]
pub extern "C" fn rt_read_line(buf: u64, cap: u64) -> u64 {
    const EOF: u64 = u64::MAX;
    let cap = cap as usize;
    if cap == 0 {
        return EOF;
    }
    let dst = unsafe { std::slice::from_raw_parts_mut(buf as *mut u8, cap) };

    with_current_io(|io| match io {
        Io::Buffered { input, in_cursor, .. } => {
            if *in_cursor >= input.len() {
                return EOF;
            }
            // Find next LF (or end-of-input).
            let start = *in_cursor;
            let rest = &input[start..];
            let lf_off = rest.iter().position(|&b| b == b'\n');
            let line_end = match lf_off {
                Some(off) => start + off,
                None => input.len(),
            };
            let n = (line_end - start).min(cap);
            dst[..n].copy_from_slice(&input[start..start + n]);
            // Advance cursor past the line + its LF (if any).
            *in_cursor = match lf_off {
                Some(_) => line_end + 1,
                None => line_end,
            };
            // Strip a trailing CR (handles CRLF inputs).
            let mut count = n as u64;
            if count > 0 && dst[count as usize - 1] == b'\r' {
                count -= 1;
            }
            count
        }
        Io::Live { .. } => {
            use std::io::{self, BufRead};
            let stdin = io::stdin();
            let mut handle = stdin.lock();
            let mut line = String::new();
            match handle.read_line(&mut line) {
                Ok(0) => EOF,
                Ok(_) => {
                    let bytes = line.as_bytes();
                    let mut len = bytes.len();
                    if len > 0 && bytes[len - 1] == b'\n' {
                        len -= 1;
                    }
                    if len > 0 && bytes[len - 1] == b'\r' {
                        len -= 1;
                    }
                    let n = len.min(cap);
                    dst[..n].copy_from_slice(&bytes[..n]);
                    n as u64
                }
                Err(_) => EOF,
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn rt_read_key() -> u64 {
    with_current_io(|io| match io {
        Io::Buffered { input, in_cursor, pending_key, .. } => {
            if let Some(byte) = pending_key.take() {
                return byte as u64;
            }
            if *in_cursor >= input.len() {
                return 0;
            }
            let byte = input[*in_cursor];
            *in_cursor += 1;
            byte as u64
        }
        Io::Live { pending_key } => {
            if let Some(byte) = pending_key.take() {
                return byte as u64;
            }
            use std::io::Read;

            let stdin = std::io::stdin();
            let mut handle = stdin.lock();
            let mut buf = [0u8; 1];
            match handle.read_exact(&mut buf) {
                Ok(()) => buf[0] as u64,
                Err(_) => 0,
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn rt_key_q() -> u64 {
    with_current_io(|io| match io {
        Io::Buffered { input, in_cursor, pending_key, .. } => {
            if pending_key.is_some() {
                return u64::MAX;
            }
            if *in_cursor >= input.len() {
                return 0;
            }
            *pending_key = Some(input[*in_cursor]);
            *in_cursor += 1;
            u64::MAX
        }
        Io::Live { pending_key } => {
            if pending_key.is_some() {
                return u64::MAX;
            }

            #[cfg(windows)]
            unsafe {
                if _kbhit() == 0 {
                    return 0;
                }
                let wide = _getwch();
                if wide <= 0xFF {
                    *pending_key = Some(wide as u8);
                    return u64::MAX;
                }
            }

            0
        }
    })
}

#[no_mangle]
pub extern "C" fn rt_to_float(addr: u64, len: u64, out_bits: u64) -> u64 {
    if len == 0 || out_bits == 0 {
        return 0;
    }

    let bytes = unsafe { std::slice::from_raw_parts(addr as *const u8, len as usize) };
    let Ok(text) = std::str::from_utf8(bytes) else {
        return 0;
    };
    let Some(normalized) = normalize_float_token(text) else {
        return 0;
    };
    let Ok(value) = normalized.parse::<f64>() else {
        return 0;
    };

    unsafe { (out_bits as *mut u64).write_unaligned(value.to_bits()) };
    u64::MAX
}

// ── File include support ──────────────────────────────────────────────
//
// `included` ( c-addr u -- ) reads a Forth source file and evaluates it.
// We implement this without re-entering the JIT by reading the file into
// a Rust-owned Vec and exposing its address+len to Forth, which then calls
// the existing `evaluate` word. A stack allows nested includes: each call
// to `rt_slurp_file` pushes a new Vec; `rt_slurp_pop` releases the top.

thread_local! {
    static SLURP_STACK: RefCell<Vec<Vec<u8>>> = RefCell::new(Vec::new());
}

/// Read file at path (c-addr u) into a Rust-owned Vec, push it onto
/// SLURP_STACK, and return a pointer to the content (stable until
/// `rt_slurp_pop` is called). Returns 0 on any error (file not found,
/// UTF-8 etc.).
#[no_mangle]
pub extern "C" fn rt_slurp_file(path_addr: u64, path_len: u64) -> u64 {
    let path_bytes =
        unsafe { std::slice::from_raw_parts(path_addr as *const u8, path_len as usize) };
    let Ok(path_str) = std::str::from_utf8(path_bytes) else {
        return 0;
    };
    match std::fs::read(path_str.trim()) {
        Ok(bytes) => SLURP_STACK.with(|s| {
            let mut stack = s.borrow_mut();
            stack.push(bytes);
            stack.last().map(|v| v.as_ptr() as u64).unwrap_or(0)
        }),
        Err(_) => 0,
    }
}

/// Return the byte length of the top slurped file (0 if stack is empty).
#[no_mangle]
pub extern "C" fn rt_slurp_len() -> u64 {
    SLURP_STACK.with(|s| {
        s.borrow().last().map(|v| v.len() as u64).unwrap_or(0)
    })
}

/// Pop (and free) the top slurped file from the stack.
#[no_mangle]
pub extern "C" fn rt_slurp_pop() -> u64 {
    SLURP_STACK.with(|s| {
        s.borrow_mut().pop();
        0
    })
}

// ── GC runtime functions ─────────────────────────────────────────────
//
// V1b plumbing.  These get called from the kernel's GC primitives
// (`vec-alloc-floats!`, `(gc)`, …) via the same @extern mechanism
// the rest of the kernel uses for I/O and number parsing.
//
// All addresses are absolute (the kernel computes them from UP +
// offset where needed).  Return value: 0 on success, u64::MAX on
// error.

use crate::gc;

/// Threshold above which an allocation is routed through paged_gc's
/// large-object path (which can fail for lack of contiguous free
/// pages even when the total free count is enough — see #9 in
/// forth_gc_needs.md).  Mirrors `newgc_core::page_heap::PAGE_SIZE_CELLS`.
const PAGE_SIZE_CELLS: u64 = 8192;

/// Allocate a FloatVec of `n_cells` payload cells, store the tagged
/// pointer at `*slot_addr`.  `slot_addr` is the absolute address of
/// a HEAPPTR slot (caller's responsibility to ensure it's inside
/// `user_HEAPPTR_REGION`).
///
/// On allocation failure for a large object (> 1 page payload), we
/// run a `collect_full` (Tenured compaction) and retry once before
/// surfacing failure — this rescues the fragmentation-defeat case
/// where total free pages are enough but no contiguous run is.
///
/// Returns 0 on success, u64::MAX on allocation failure (heap full
/// even after compaction, or `n_cells > u32::MAX`).
#[no_mangle]
pub extern "C" fn rt_vec_alloc_floats(up: u64, n_cells: u64, slot_addr: u64) -> u64 {
    if n_cells > u32::MAX as u64 {
        eprintln!("vec-alloc-floats!: requested length {n_cells} exceeds u32::MAX");
        return u64::MAX;
    }
    let n = n_cells as u32;
    if let Some(tagged) = gc::alloc_floatvec(n) {
        unsafe { *(slot_addr as *mut u64) = tagged; }
        return 0;
    }
    if n_cells > PAGE_SIZE_CELLS {
        let regions = gc_root_regions(up);
        unsafe { gc::collect_full(&regions); }
        if let Some(tagged) = gc::alloc_floatvec(n) {
            unsafe { *(slot_addr as *mut u64) = tagged; }
            return 0;
        }
    }
    eprintln!("vec-alloc-floats!: out of GC heap (requested {n_cells} cells)");
    u64::MAX
}

/// Allocate a RefVec of `n_cells` (all initialised to nil), store
/// the tagged pointer at `*slot_addr`.  Fragmentation-retry path
/// mirrors `rt_vec_alloc_floats`.
#[no_mangle]
pub extern "C" fn rt_vec_alloc_refs(up: u64, n_cells: u64, slot_addr: u64) -> u64 {
    if n_cells > u32::MAX as u64 {
        eprintln!("vec-alloc-refs!: requested length {n_cells} exceeds u32::MAX");
        return u64::MAX;
    }
    let n = n_cells as u32;
    if let Some(tagged) = gc::alloc_refvec(n) {
        unsafe { *(slot_addr as *mut u64) = tagged; }
        return 0;
    }
    if n_cells > PAGE_SIZE_CELLS {
        let regions = gc_root_regions(up);
        unsafe { gc::collect_full(&regions); }
        if let Some(tagged) = gc::alloc_refvec(n) {
            unsafe { *(slot_addr as *mut u64) = tagged; }
            return 0;
        }
    }
    eprintln!("vec-alloc-refs!: out of GC heap (requested {n_cells} cells)");
    u64::MAX
}

/// Read the (base, next) pair for both GC root regions —
/// HEAPPTR and LITERAL — from the user area.  Returns
/// `(heapptr_pair, literal_pair)`.  Either pair may be (b, b)
/// meaning empty.
fn gc_root_regions(up: u64) -> [(u64, u64); 2] {
    let heapptr_next = unsafe { *((up + crate::USER_HEAPPTR_NEXT) as *const u64) };
    let heapptr_base = up + crate::USER_HEAPPTR_BASE;
    let literal_next = unsafe { *((up + crate::USER_LITERAL_NEXT) as *const u64) };
    let literal_base = up + crate::USER_LITERAL_BASE;
    [(heapptr_base, heapptr_next), (literal_base, literal_next)]
}

/// Run a major GC.  Walks BOTH the HEAPPTR region and the LITERAL
/// region as roots (V2s: compile-time string literals live in the
/// second region).  Caller passes `up` (the UP register value).
#[no_mangle]
pub extern "C" fn rt_gc_collect(up: u64) -> u64 {
    let regions = gc_root_regions(up);
    for (base, next) in regions {
        if next < base {
            eprintln!("(gc): root region NEXT (0x{next:x}) is below BASE \
                       (0x{base:x}) — probable user-area corruption");
            return u64::MAX;
        }
    }
    unsafe { gc::collect_major(&regions); }
    0
}

/// Run a minor GC.  Same root set as `rt_gc_collect`.
#[no_mangle]
pub extern "C" fn rt_gc_collect_minor(up: u64) -> u64 {
    let regions = gc_root_regions(up);
    for (base, next) in regions {
        if next < base {
            return u64::MAX;
        }
    }
    unsafe { gc::collect_minor(&regions); }
    0
}

/// True (returns 1) when paged_gc's allocation budget has been
/// exhausted and the next allocation should be preceded by a
/// minor GC.  Retained for diagnostics / future opt-out callers;
/// the kernel allocators now go through `rt_gc_auto_step` instead.
///
/// Returns 0 when no collection is needed.  Never errors.
#[no_mangle]
pub extern "C" fn rt_gc_should_collect() -> u64 {
    if gc::should_collect() { 1 } else { 0 }
}

/// Combined auto-trigger: if `should_collect()` is true, run
/// `collect_auto` (which chooses minor vs major based on tenure
/// pressure — see docs/forth_gc_needs.md item #7).  Otherwise
/// no-op.  Folds the previous "rt_gc_should_collect + maybe
/// rt_gc_collect_minor" two-call pattern into one extern call
/// from kernel-side allocators.
///
/// Returns 0 always; the auto-cycle's outcome is observable
/// indirectly via `gc-cycle`.
#[no_mangle]
pub extern "C" fn rt_gc_auto_step(up: u64) -> u64 {
    if !gc::should_collect() {
        return 0;
    }
    let regions = gc_root_regions(up);
    unsafe { gc::collect_auto(&regions); }
    0
}

/// Full stop-the-world collection — compacts Tenured.  Called by
/// the large-object alloc retry path when `try_alloc_large`
/// failed for lack of contiguous free pages (paged_gc's page
/// allocator is linear-scan; scattered free pages can defeat a
/// multi-page request even when the total free count is enough).
/// Per docs/forth_gc_needs.md item #9.
#[no_mangle]
pub extern "C" fn rt_gc_collect_full(up: u64) -> u64 {
    let regions = gc_root_regions(up);
    for (base, next) in regions {
        if next < base {
            return u64::MAX;
        }
    }
    unsafe { gc::collect_full(&regions); }
    0
}

/// Current value of the GC cycle counter — monotonically incremented
/// by one on every successful collection (major or minor).  Exposed
/// as the Forth word `gc-cycle ( -- n )`.  Reset to 0 by session
/// reset between harness tests.
#[no_mangle]
pub extern "C" fn rt_gc_cycle_count() -> u64 {
    gc::gc_cycle_count()
}

// ── Managed strings (V2s) ────────────────────────────────────────────
//
// See docs/strings_design.md for the full surface.  Stage A: allocate
// a String from arbitrary bytes, byte-equality.  The other operations
// (`$len`, `$>addr`, `@$`, `!$`) are pure assembly — see
// kernel/strings.masm.

/// Allocate a new `String` GC object and copy `len` bytes from
/// `src_addr` into its payload.  `src_addr` may point anywhere in
/// readable memory (PAD, dictionary heap, slurped-file buffer); the
/// copy is independent of the source after this call returns.
///
/// Same fragmentation-retry path as `rt_vec_alloc_*` for strings
/// whose payload exceeds one page (~64 KB).
///
/// Returns the tagged pointer (low 3 bits = `TAG_STRING`) on
/// success, `u64::MAX` on allocation failure or oversized input.
///
/// # Safety
/// `src_addr..src_addr+len` must be readable.  Kernel-side `>$`
/// guarantees this — it gets the (c-addr, u) pair directly from
/// the Forth data stack.
#[no_mangle]
pub extern "C" fn rt_string_from_bytes(up: u64, src_addr: u64, len: u64) -> u64 {
    if len > u32::MAX as u64 {
        eprintln!(">$: requested length {len} exceeds u32::MAX");
        return u64::MAX;
    }
    let n = len as u32;
    let tagged = match gc::alloc_string(n) {
        Some(t) => t,
        None => {
            // Retry via collect_full if this is a large object.
            let payload_cells = ((len + 7) / 8) as u64;
            if payload_cells > PAGE_SIZE_CELLS {
                let regions = gc_root_regions(up);
                unsafe { gc::collect_full(&regions); }
                match gc::alloc_string(n) {
                    Some(t) => t,
                    None => {
                        eprintln!(">$: out of GC heap (requested {len} bytes, post-compaction retry failed)");
                        return u64::MAX;
                    }
                }
            } else {
                eprintln!(">$: out of GC heap (requested {len} bytes)");
                return u64::MAX;
            }
        }
    };
    if len > 0 {
        // Payload lives at base + 8 (one header cell).  Strip tag
        // to get the base.
        let base = tagged & !7;
        let dst = (base + 8) as *mut u8;
        unsafe {
            std::ptr::copy_nonoverlapping(src_addr as *const u8, dst, len as usize);
        }
    }
    tagged
}

// ── MutStringBuilder runtime (V2s stage C1) ──────────────────────────

/// Read (length, capacity) from a builder's 2-cell header.
/// Caller MUST have verified the tag.
#[inline]
unsafe fn builder_header(base: u64) -> (u32, u32) {
    let hdr = unsafe { *(base as *const u64) };
    let cap = unsafe { *((base + 8) as *const u64) };
    let length = ((hdr >> 5) & 0xFF_FFFF) as u32;
    (length, cap as u32)
}

/// Write a new length into a builder's header word, preserving the
/// type and GC bits.
#[inline]
unsafe fn builder_set_length(base: u64, new_length: u32) {
    let p = base as *mut u64;
    let hdr = unsafe { *p };
    // Clear bits 5..29 (length field), OR in the new length.
    let cleared = hdr & !(0xFF_FFFF << 5);
    let new_hdr = cleared | ((new_length as u64 & 0xFF_FFFF) << 5);
    unsafe { *p = new_hdr; }
}

/// Allocate a fresh `MutStringBuilder` with the given capacity in
/// bytes.  Returns the tagged pointer, or `u64::MAX` on failure.
/// Auto-trigger + fragmentation-retry mirror the other allocators.
#[no_mangle]
pub extern "C" fn rt_sb_new(up: u64, capacity_bytes: u64) -> u64 {
    if capacity_bytes > u32::MAX as u64 {
        eprintln!("sb-new: capacity {capacity_bytes} exceeds u32::MAX");
        return u64::MAX;
    }
    let cap = capacity_bytes as u32;
    let tagged = match gc::alloc_builder(cap) {
        Some(t) => t,
        None => {
            // Builder body cells = 2 + ceil(cap/8). Threshold against
            // PAGE_SIZE_CELLS for the fragmentation-retry decision.
            let payload_cells = ((capacity_bytes + 7) / 8) as u64;
            let total_cells = 2 + payload_cells;
            if total_cells > PAGE_SIZE_CELLS {
                let regions = gc_root_regions(up);
                unsafe { gc::collect_full(&regions); }
                match gc::alloc_builder(cap) {
                    Some(t) => t,
                    None => {
                        eprintln!("sb-new: out of GC heap (cap={capacity_bytes}, post-compaction retry failed)");
                        return u64::MAX;
                    }
                }
            } else {
                eprintln!("sb-new: out of GC heap (cap={capacity_bytes})");
                return u64::MAX;
            }
        }
    };
    tagged
}

/// Append `len` bytes from `src_addr` to the builder identified by
/// `builder_tagged`.  Caller MUST have verified the tag is
/// `TAG_BUILDER`.  Returns 0 on success or `u64::MAX` if the
/// append would overflow capacity (used as the -2062 throw signal
/// in the kernel-side wrapper).
#[no_mangle]
pub extern "C" fn rt_sb_append_bytes(
    builder_tagged: u64,
    src_addr: u64,
    len: u64,
) -> u64 {
    if len == 0 {
        return 0;
    }
    let base = builder_tagged & !7;
    let (length, capacity) = unsafe { builder_header(base) };
    let new_length = length as u64 + len;
    if new_length > capacity as u64 {
        eprintln!("sb-append: would overflow capacity \
                   ({length} + {len} > {capacity})");
        return u64::MAX;
    }
    let dst = (base + 16 + length as u64) as *mut u8;
    unsafe {
        std::ptr::copy_nonoverlapping(src_addr as *const u8, dst, len as usize);
        builder_set_length(base, new_length as u32);
    }
    0
}

/// Append the UTF-8 encoding of `codepoint` to the builder.  Returns
/// 0 on success, `u64::MAX` on capacity overflow or an invalid
/// codepoint (surrogate or > U+10FFFF).
#[no_mangle]
pub extern "C" fn rt_sb_append_codepoint(
    builder_tagged: u64,
    codepoint: u64,
) -> u64 {
    let cp = match char::from_u32(codepoint as u32) {
        Some(c) => c,
        None => {
            eprintln!("sb-append-c: invalid codepoint {codepoint:#x}");
            return u64::MAX;
        }
    };
    let mut buf = [0u8; 4];
    let encoded = cp.encode_utf8(&mut buf);
    rt_sb_append_bytes(
        builder_tagged,
        encoded.as_ptr() as u64,
        encoded.len() as u64,
    )
}

/// Append the decimal representation of a signed integer `n` to
/// the builder.  Returns 0 on success, `u64::MAX` on overflow.
#[no_mangle]
pub extern "C" fn rt_sb_append_int(builder_tagged: u64, n: u64) -> u64 {
    let s = (n as i64).to_string();
    rt_sb_append_bytes(
        builder_tagged,
        s.as_ptr() as u64,
        s.len() as u64,
    )
}

/// Finalise the builder to a fresh immutable `String`.  Allocates
/// a new String of `builder.length` bytes, copies the payload,
/// then resets `builder.length` to 0 (capacity retained) per the
/// V2s design — the builder remains usable.  Returns the tagged
/// String pointer, or `u64::MAX` on allocation failure.
#[no_mangle]
pub extern "C" fn rt_sb_to_string(up: u64, builder_tagged: u64) -> u64 {
    let base = builder_tagged & !7;
    let (length, _capacity) = unsafe { builder_header(base) };
    let payload_addr = base + 16;
    let tagged = rt_string_from_bytes(up, payload_addr, length as u64);
    if tagged == u64::MAX {
        return u64::MAX;
    }
    unsafe { builder_set_length(base, 0); }
    tagged
}

/// Compile-mode helper for `S$"`.  Allocates a LITERAL slot,
/// allocates a fresh `String` GC object copying `len` bytes from
/// `src_addr`, stores the tagged pointer into the literal slot,
/// and emits a 22-byte stub at HERE that pushes `[slot_addr]` at
/// runtime.  HERE is advanced past the stub.
///
/// Returns 0 on success; `u64::MAX` on allocation failure or
/// LITERAL region overflow.  Errors are surfaced to the caller as
/// throws (caller picks the throw code: -2058 for region overflow,
/// -2059 for alloc failure).  Stderr-logged for the developer.
///
/// # Safety
/// `up` must be a valid Forth user-area pointer; `src_addr` ..
/// `src_addr + len` must be readable.
#[no_mangle]
pub extern "C" fn rt_s_literal_compile_at_here(
    up: u64,
    src_addr: u64,
    len: u64,
) -> u64 {
    // Bounds-check the LITERAL region.
    let literal_base = up + crate::USER_LITERAL_BASE;
    let literal_limit = literal_base + crate::LITERAL_REGION_SIZE;
    let next_addr_ptr = (up + crate::USER_LITERAL_NEXT) as *mut u64;
    let cur_next = unsafe { *next_addr_ptr };
    if cur_next + 8 > literal_limit {
        eprintln!(r#"S$": LITERAL region full (next=0x{cur_next:x}, limit=0x{literal_limit:x})"#);
        return u64::MAX;
    }

    // Allocate a String and copy bytes.
    let tagged = rt_string_from_bytes(up, src_addr, len);
    if tagged == u64::MAX {
        return u64::MAX;
    }

    // Reserve the slot, store the tagged ptr, bump LITERAL_NEXT.
    let slot_addr = cur_next;
    unsafe {
        *(slot_addr as *mut u64) = tagged;
        *next_addr_ptr = cur_next + 8;
    }

    // Emit the runtime stub at HERE.  Stub pushes [slot_addr]
    // and falls through to the next compiled word in the colon
    // body — no `ret`, because the colon body is a sequence of
    // inline calls/operations, not a series of leaf words.
    //   mov [rbp-8], rax         (4)  48 89 45 F8   - spill TOS
    //   mov rax, imm64=slot_addr (10) 48 B8 + 8     - rax = slot addr
    //   mov rax, [rax]           (3)  48 8B 00      - rax = *slot
    //   sub rbp, 8                (4)  48 83 ED 08  - lower DSP
    // Total = 21 bytes.  Control falls through.
    let here = unsafe { *((up + RT_USER_HERE) as *const u64) };
    let dst = here as *mut u8;
    unsafe {
        *dst.add(0)  = 0x48; *dst.add(1)  = 0x89;
        *dst.add(2)  = 0x45; *dst.add(3)  = 0xF8;
        *dst.add(4)  = 0x48; *dst.add(5)  = 0xB8;
        write_u64_le(dst.add(6), slot_addr);
        *dst.add(14) = 0x48; *dst.add(15) = 0x8B; *dst.add(16) = 0x00;
        *dst.add(17) = 0x48; *dst.add(18) = 0x83;
        *dst.add(19) = 0xED; *dst.add(20) = 0x08;
        *((up + RT_USER_HERE) as *mut u64) = here + 21;
    }
    0
}

/// Byte-compare two `String` payloads.  Returns `u64::MAX` (-1, i.e.
/// Forth true) if the bytes match exactly, `0` otherwise.
///
/// The kernel-side `$=` is responsible for tag-checking both inputs
/// before calling — this function trusts that both pointers carry
/// `TAG_STRING` and that the headers' length fields are
/// authoritative.
///
/// # Safety
/// Both `tagged_a` and `tagged_b` must be valid `String` tagged
/// pointers.  Nil or wrong-typed inputs will deref invalid memory.
#[no_mangle]
pub extern "C" fn rt_string_bytes_equal(tagged_a: u64, tagged_b: u64) -> u64 {
    // Fast path: same object.
    if tagged_a == tagged_b {
        return u64::MAX;
    }
    let base_a = (tagged_a & !7) as *const u64;
    let base_b = (tagged_b & !7) as *const u64;
    // Header layout: bits[0..5]=type, [5..29]=length, [29..]=GC.
    // Compare lengths first.
    let len_a = unsafe { (*base_a >> 5) & 0xFF_FFFF };
    let len_b = unsafe { (*base_b >> 5) & 0xFF_FFFF };
    if len_a != len_b {
        return 0;
    }
    if len_a == 0 {
        return u64::MAX;
    }
    let payload_a = unsafe { (base_a as *const u8).add(8) };
    let payload_b = unsafe { (base_b as *const u8).add(8) };
    let slice_a = unsafe { std::slice::from_raw_parts(payload_a, len_a as usize) };
    let slice_b = unsafe { std::slice::from_raw_parts(payload_b, len_a as usize) };
    if slice_a == slice_b { u64::MAX } else { 0 }
}

// ── LET DSL compilation ──────────────────────────────────────────────
//
// `rt_let_compile(up)` is called by the kernel's immediate `LET` word.
// It reads the LET source from the current input buffer up to the next
// `END` token, compiles it via [`crate::let_lang`], JITs the result in
// a fresh module (kept alive in `LET_JITS`), and emits a Win64
// trampoline at HERE that loads inputs from the Forth FP stack,
// invokes the compiled function, and adjusts FSP.
//
// Returns 0 on success or `u64::MAX` (= -1 as i64) on any error;
// error details are printed to stderr.

use std::sync::atomic::{AtomicUsize, Ordering};
use wfasm::Jit;

use crate::let_lang;

// User-area offsets — keep in sync with macros.masm.
const RT_USER_SOURCE_ADDR: u64 = 0x30;
const RT_USER_SOURCE_LEN:  u64 = 0x38;
const RT_USER_TO_IN:       u64 = 0x40;
const RT_USER_HERE:        u64 = 0x18;
const RT_USER_FSP:         u64 = 0x1218;

thread_local! {
    /// Compiled LET functions live in their own JIT modules.  We keep
    /// every Jit alive for the duration of the session so the executable
    /// pages don't get freed under us when a colon definition still
    /// holds a CALL to the compiled function pointer.
    static LET_JITS: RefCell<Vec<Jit>> = RefCell::new(Vec::new());
}

/// Counter for generating unique LET function names. Persists for the
/// process lifetime; we don't reuse names because old Jits may still hold
/// the old name (and that's fine, but a fresh counter avoids confusion).
static LET_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Drop every LET-compiled Jit. Called by session reset between tests.
pub fn reset_let_session() {
    LET_JITS.with(|j| j.borrow_mut().clear());
}

/// libm functions the LET codegen may reference, with their declared
/// arity.  Resolved via GetProcAddress on ucrtbase.dll (the Windows
/// C runtime); missing ones are simply not registered, so a LET that
/// doesn't use them still compiles fine.
const LIBM_FUNCTIONS: &[(&str, usize)] = &[
    ("sin", 1), ("cos", 1), ("tan", 1),
    ("asin", 1), ("acos", 1), ("atan", 1),
    ("exp", 1), ("log", 1), ("log2", 1), ("log10", 1),
    ("atan2", 2), ("pow", 2), ("hypot", 2), ("fmod", 2),
];

#[cfg(windows)]
mod win_libm {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_void};

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryW(name: *const u16) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
    }

    /// Get a handle to ucrtbase.dll. It's a process-global module — the
    /// C runtime is already loaded — so LoadLibraryW just bumps the
    /// refcount.  Cached per-thread to avoid hammering LoadLibrary on
    /// every LET.
    pub fn ucrtbase_handle() -> *mut c_void {
        thread_local! {
            static HANDLE: std::cell::Cell<*mut c_void> = const { std::cell::Cell::new(std::ptr::null_mut()) };
        }
        HANDLE.with(|h| {
            let cur = h.get();
            if !cur.is_null() {
                return cur;
            }
            // UTF-16 "ucrtbase.dll\0"
            let wname: Vec<u16> = "ucrtbase.dll".encode_utf16().chain(std::iter::once(0)).collect();
            let new = unsafe { LoadLibraryW(wname.as_ptr()) };
            h.set(new);
            new
        })
    }

    /// GetProcAddress wrapper. Returns None on missing symbol.
    pub fn proc_addr(module: *mut c_void, name: &str) -> Option<*mut c_void> {
        if module.is_null() {
            return None;
        }
        let cname = CString::new(name).ok()?;
        let addr = unsafe { GetProcAddress(module, cname.as_ptr()) };
        if addr.is_null() { None } else { Some(addr) }
    }
}

/// Resolve every libm function in LIBM_FUNCTIONS via GetProcAddress.
/// Returns the (name → host address) table used by the LET codegen to
/// bake direct `mov rax, addr; call rax` sequences into compiled LETs.
/// Missing symbols are simply omitted; the codegen produces an error
/// only if the user's LET references one that's absent.
#[cfg(windows)]
pub fn libm_address_table() -> let_lang::LibmTable {
    let mut t = let_lang::LibmTable::new();
    let h = win_libm::ucrtbase_handle();
    if h.is_null() { return t; }
    for &(name, _arity) in LIBM_FUNCTIONS {
        if let Some(addr) = win_libm::proc_addr(h, name) {
            t.insert(name.to_string(), addr as u64);
        }
    }
    t
}

#[cfg(not(windows))]
pub fn libm_address_table() -> let_lang::LibmTable {
    let_lang::LibmTable::new()
}

unsafe fn read_u64(addr: u64) -> u64 { unsafe { *(addr as *const u64) } }
unsafe fn write_u64(addr: u64, val: u64) { unsafe { *(addr as *mut u64) = val } }

/// Compile a LET form from the current input buffer.
///
/// # Safety
/// `up` must point to a valid Forth user area whose SOURCE_ADDR /
/// SOURCE_LEN / TO_IN / HERE fields are correctly maintained by the
/// kernel.
#[no_mangle]
pub extern "C" fn rt_let_compile(up: u64) -> u64 {
    match unsafe { try_compile_let(up) } {
        Ok(()) => 0,
        Err(msg) => {
            eprintln!("LET compile error: {msg}");
            u64::MAX
        }
    }
}

unsafe fn try_compile_let(up: u64) -> Result<(), String> {
    let src_base = unsafe { read_u64(up + RT_USER_SOURCE_ADDR) };
    let src_len  = unsafe { read_u64(up + RT_USER_SOURCE_LEN)  };
    let to_in    = unsafe { read_u64(up + RT_USER_TO_IN)        };

    if to_in > src_len {
        return Err(format!("TO_IN ({to_in}) past SOURCE_LEN ({src_len})"));
    }

    let remaining = unsafe {
        std::slice::from_raw_parts(
            (src_base + to_in) as *const u8,
            (src_len - to_in) as usize,
        )
    };

    let (body_bytes, consumed) = find_end_token(remaining)
        .ok_or_else(|| "no closing 'END' token in LET body".to_string())?;
    let body_str = std::str::from_utf8(body_bytes)
        .map_err(|_| "LET body is not UTF-8".to_string())?;

    // Our parser starts at `LET`; the keyword was already consumed by
    // the Forth interpreter before dispatching to our word.  Prepend
    // it back, plus the closing END, so parser sees a complete form.
    let source = format!("LET{body_str}END");

    let counter = LET_COUNTER.fetch_add(1, Ordering::SeqCst);
    let fn_name = format!("let_user_{counter:04}");

    let libm_table = libm_address_table();
    let compiled = let_lang::compile(&source, &fn_name, &libm_table)
        .map_err(|e| e.to_string())?;

    // Compile into a fresh JIT module so the main kernel module stays
    // frozen and we don't fight MCJIT's whole-module finalization rule.
    let mut jit = Jit::new(&format!("let_mod_{counter:04}"))
        .map_err(|e| format!("Jit::new: {e:?}"))?;
    jit.add_asm(&compiled.asm_text)
        .map_err(|e| format!("add_asm: {e:?}\nasm was:\n{}", compiled.asm_text))?;
    jit.declare_fn(&compiled.fn_name, 0)
        .map_err(|e| format!("declare_fn({}): {e:?}", compiled.fn_name))?;
    let fn_addr = jit.lookup_addr(&compiled.fn_name)
        .map_err(|e| format!("lookup_addr({}): {e:?}", compiled.fn_name))?;

    LET_JITS.with(|j| j.borrow_mut().push(jit));

    let here = unsafe { read_u64(up + RT_USER_HERE) };
    let trampoline_len = unsafe {
        emit_let_trampoline(here, fn_addr, compiled.n_inputs, compiled.n_outputs)
    };
    unsafe { write_u64(up + RT_USER_HERE, here + trampoline_len as u64); }
    unsafe { write_u64(up + RT_USER_TO_IN, to_in + consumed as u64); }
    Ok(())
}

/// Find the next "END" token in `src` (whitespace-delimited).
/// Returns (body-before-END, total-bytes-consumed-including-END).
fn find_end_token(src: &[u8]) -> Option<(&[u8], usize)> {
    let mut i = 0;
    while i + 3 <= src.len() {
        if &src[i..i + 3] == b"END" {
            let prev_ok = i == 0 || !is_ident_byte(src[i - 1]);
            let next_ok = i + 3 == src.len() || !is_ident_byte(src[i + 3]);
            if prev_ok && next_ok {
                return Some((&src[..i], i + 3));
            }
        }
        i += 1;
    }
    None
}

fn is_ident_byte(b: u8) -> bool { b.is_ascii_alphanumeric() || b == b'_' }

/// Emit Win64 trampoline at `here` calling fn_addr with rcx = FSP and
/// rdx = FSP + delta, then bumping FSP by delta where delta = (n_in - n_out)*8.
/// Returns the number of bytes emitted.
unsafe fn emit_let_trampoline(here: u64, fn_addr: u64, n_in: usize, n_out: usize) -> usize {
    let delta: i64 = (n_in as i64 - n_out as i64) * 8;
    let delta_i32: i32 = delta as i32;
    let dst = here as *mut u8;
    let mut p: usize = 0;

    // mov rcx, qword ptr [rbx + USER_FSP] :: 48 8B 8B disp32
    unsafe {
        *dst.add(p) = 0x48; p += 1;
        *dst.add(p) = 0x8B; p += 1;
        *dst.add(p) = 0x8B; p += 1;
        write_i32(dst.add(p), RT_USER_FSP as i32); p += 4;
    }

    // rdx = rcx + delta
    if delta == 0 {
        unsafe {
            // mov rdx, rcx :: 48 89 CA
            *dst.add(p) = 0x48; p += 1;
            *dst.add(p) = 0x89; p += 1;
            *dst.add(p) = 0xCA; p += 1;
        }
    } else if (-128..=127).contains(&delta) {
        unsafe {
            // lea rdx, [rcx + imm8] :: 48 8D 51 imm8
            *dst.add(p) = 0x48; p += 1;
            *dst.add(p) = 0x8D; p += 1;
            *dst.add(p) = 0x51; p += 1;
            *dst.add(p) = (delta as i8) as u8; p += 1;
        }
    } else {
        unsafe {
            // lea rdx, [rcx + imm32] :: 48 8D 91 imm32
            *dst.add(p) = 0x48; p += 1;
            *dst.add(p) = 0x8D; p += 1;
            *dst.add(p) = 0x91; p += 1;
            write_i32(dst.add(p), delta_i32); p += 4;
        }
    }

    // mov r12, rsp :: 49 89 E4
    unsafe {
        *dst.add(p) = 0x49; p += 1;
        *dst.add(p) = 0x89; p += 1;
        *dst.add(p) = 0xE4; p += 1;
        // and rsp, -16 :: 48 83 E4 F0
        *dst.add(p) = 0x48; p += 1;
        *dst.add(p) = 0x83; p += 1;
        *dst.add(p) = 0xE4; p += 1;
        *dst.add(p) = 0xF0; p += 1;
        // sub rsp, 32 :: 48 83 EC 20
        *dst.add(p) = 0x48; p += 1;
        *dst.add(p) = 0x83; p += 1;
        *dst.add(p) = 0xEC; p += 1;
        *dst.add(p) = 0x20; p += 1;
        // mov rax, imm64 :: 48 B8 [8 bytes]
        *dst.add(p) = 0x48; p += 1;
        *dst.add(p) = 0xB8; p += 1;
        write_u64_le(dst.add(p), fn_addr); p += 8;
        // call rax :: FF D0
        *dst.add(p) = 0xFF; p += 1;
        *dst.add(p) = 0xD0; p += 1;
        // mov rsp, r12 :: 4C 89 E4
        *dst.add(p) = 0x4C; p += 1;
        *dst.add(p) = 0x89; p += 1;
        *dst.add(p) = 0xE4; p += 1;
    }

    // Adjust FSP by delta.
    if delta == 0 {
        // nothing to emit
    } else if (-128..=127).contains(&delta) {
        unsafe {
            // add qword ptr [rbx + USER_FSP], imm8 :: 48 83 83 disp32 imm8
            *dst.add(p) = 0x48; p += 1;
            *dst.add(p) = 0x83; p += 1;
            *dst.add(p) = 0x83; p += 1;
            write_i32(dst.add(p), RT_USER_FSP as i32); p += 4;
            *dst.add(p) = (delta as i8) as u8; p += 1;
        }
    } else {
        unsafe {
            // add qword ptr [rbx + USER_FSP], imm32 :: 48 81 83 disp32 imm32
            *dst.add(p) = 0x48; p += 1;
            *dst.add(p) = 0x81; p += 1;
            *dst.add(p) = 0x83; p += 1;
            write_i32(dst.add(p), RT_USER_FSP as i32); p += 4;
            write_i32(dst.add(p), delta_i32); p += 4;
        }
    }

    p
}

unsafe fn write_i32(dst: *mut u8, val: i32) {
    let bytes = val.to_le_bytes();
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, 4); }
}

unsafe fn write_u64_le(dst: *mut u8, val: u64) {
    let bytes = val.to_le_bytes();
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, 8); }
}

// ── CODE: DSL compilation ────────────────────────────────────────────
//
// `rt_code_compile_body(up)` is the worker behind the `CODE:` immediate
// word.  It reads the assembly source from the current input buffer up
// to the next `;CODE` token, wraps it in a `proc(...)` / `endp()` pair,
// hands it to a thread-local JASM `Assembler` (preloaded with
// `macros.masm`, so the user's source can use `proc/endp/next/pushd/stk`
// etc. naturally), JIT-compiles into a fresh module, and returns the
// resulting function address.  The kernel's `CODE:` word then builds
// the dict header and emits a 12-byte JMP trampoline at HERE that
// transfers control to the compiled function.
//
// Returns the function address on success, 0 on any error (details
// printed to stderr).

const MACROS_SOURCE: &str = include_str!("../kernel/macros.masm");

thread_local! {
    /// Each compiled CODE: word lives in its own JIT module.  We keep
    /// the Jit alive for the session lifetime so its executable memory
    /// stays mapped while colon definitions still reference the
    /// function via the trampoline.
    static CODE_JITS: RefCell<Vec<Jit>> = RefCell::new(Vec::new());

    /// Shared JASM Assembler pre-loaded with `macros.masm`. Stored as
    /// `Option` so we lazily initialise on first use — at that point the
    /// kernel layout is already established and macros.masm parses cleanly.
    static CODE_ASSEMBLER: RefCell<Option<wfasm::Assembler>> = const { RefCell::new(None) };
}

static CODE_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn reset_code_session() {
    CODE_JITS.with(|j| j.borrow_mut().clear());
    // CODE_ASSEMBLER intentionally kept — re-bootstrapping macros.masm
    // for every reset would be wasteful, and its expansion-time state
    // (defines, assigns, macros) doesn't accumulate per call.
}

/// Compile the next CODE: body in the input buffer.
///
/// Returns the address of the JIT-compiled function on success, or 0
/// on any failure (the kernel surfaces this as a THROW).
#[no_mangle]
pub extern "C" fn rt_code_compile_body(up: u64) -> u64 {
    match unsafe { try_compile_code(up) } {
        Ok(addr) => addr,
        Err(msg) => {
            eprintln!("CODE: compile error: {msg}");
            0
        }
    }
}

unsafe fn try_compile_code(up: u64) -> Result<u64, String> {
    let src_base = unsafe { read_u64(up + RT_USER_SOURCE_ADDR) };
    let src_len  = unsafe { read_u64(up + RT_USER_SOURCE_LEN)  };
    let to_in    = unsafe { read_u64(up + RT_USER_TO_IN)        };

    if to_in > src_len {
        return Err(format!("TO_IN ({to_in}) past SOURCE_LEN ({src_len})"));
    }

    // Assemble the full CODE: body, which may span multiple input lines
    // when the user types it across several REPL lines.  We scan first
    // the current SOURCE buffer (rest of THIS line) and then, if no
    // `;CODE` is found there, the Io's input buffer past `in_cursor`.
    let current_tail = unsafe {
        std::slice::from_raw_parts(
            (src_base + to_in) as *const u8,
            (src_len - to_in) as usize,
        )
    };

    let body_string: String;
    let consumed_in_current: usize;
    let consumed_from_buffer: usize;

    if let Some((body, n)) = find_code_terminator(current_tail) {
        // Body fits on the current line.
        body_string = std::str::from_utf8(body)
            .map_err(|_| "CODE: body is not UTF-8".to_string())?
            .to_string();
        consumed_in_current = n;
        consumed_from_buffer = 0;
    } else {
        // Need to peek into the Io input buffer for additional lines.
        let current_tail_str = std::str::from_utf8(current_tail)
            .map_err(|_| "CODE: source is not UTF-8".to_string())?;
        let (extra, n_from_buf) = peek_until_code_terminator()
            .ok_or_else(|| "no closing ';CODE' token found before EOF".to_string())?;
        // Combine: current line tail + newline + extra
        body_string = format!("{current_tail_str}\n{extra}");
        consumed_in_current = current_tail.len();
        consumed_from_buffer = n_from_buf;
    }
    let body_str = &body_string;

    let counter = CODE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let fn_label = format!("code_user_{counter:04}");

    // Wrap in proc/endp so the user can write idiomatic kernel asm with
    // `next()` / `pushd` / `popd` / `stk(in,out)` / etc.  We auto-emit a
    // trailing `next()` (= ret) so the user doesn't have to remember it,
    // but if they wrote their own that just becomes dead bytes.
    let asm_source = format!(
        ".intel_syntax noprefix\n\
         .text\n\
         proc({fn_label})\n\
         {body_str}\n\
         next()\n\
         endp()\n",
    );

    let mc_text = with_code_assembler(|asm| -> Result<String, String> {
        asm.assemble(&format!("code_body_{counter:04}"), &asm_source)
            .map_err(|e| format!("{e}"))
    })?;

    let mut jit = Jit::new(&format!("code_mod_{counter:04}"))
        .map_err(|e| format!("Jit::new: {e:?}"))?;
    jit.add_asm(&mc_text)
        .map_err(|e| format!("add_asm: {e:?}\nasm was:\n{mc_text}"))?;
    jit.declare_fn(&fn_label, 0)
        .map_err(|e| format!("declare_fn({fn_label}): {e:?}"))?;
    let fn_addr = jit.lookup_addr(&fn_label)
        .map_err(|e| format!("lookup_addr({fn_label}): {e:?}"))?;

    CODE_JITS.with(|j| j.borrow_mut().push(jit));

    // Advance TO_IN past the consumed portion of the current line.
    unsafe {
        write_u64(up + RT_USER_TO_IN, to_in + consumed_in_current as u64);
    }
    // If we consumed lines from the Io buffer too, advance in_cursor.
    if consumed_from_buffer > 0 {
        advance_io_cursor(consumed_from_buffer);
    }
    Ok(fn_addr)
}

/// Peek into the current session's input past the kernel's `TO_IN`,
/// scanning for `;CODE`.  Returns (body_bytes_before_terminator,
/// bytes_consumed_from_buffered_input).
///
/// * Buffered mode: scans `input[in_cursor..]`, returns the consumed
///   byte count so the caller can advance `in_cursor`.
/// * Live mode: reads lines from stdin via `BufRead::read_line` until
///   it sees `;CODE`.  Stdin has been advanced past those lines as a
///   side effect, so the kernel's next refill picks up after `;CODE`.
///   `consumed` is irrelevant in this mode — returned as 0.
///
/// Returns None on EOF or read error.
fn peek_until_code_terminator() -> Option<(String, usize)> {
    // First snapshot: is the current Io Buffered or Live?  We can't
    // hold the borrow across stdin reads (would deadlock on Live mode's
    // re-entry into with_current_io somewhere downstream), so check
    // first, release, then act.
    let mode = CURRENT_IO.with(|cell| {
        cell.borrow().as_ref().map(|io| matches!(io, Io::Buffered { .. }))
    })?;

    if mode {
        // Buffered: scan the input vec directly.
        CURRENT_IO.with(|cell| {
            let borrow = cell.borrow();
            let io = borrow.as_ref()?;
            if let Io::Buffered { input, in_cursor, .. } = io {
                let rest = &input[*in_cursor..];
                let (body, consumed) = find_code_terminator(rest)?;
                let body_str = std::str::from_utf8(body).ok()?.to_string();
                Some((body_str, consumed))
            } else {
                None
            }
        })
    } else {
        // Live: read lines from stdin until we see ;CODE.  Each
        // BufRead::read_line consumes from stdin, so the kernel's
        // next refill picks up after our last consumed line.
        use std::io::{self, BufRead};
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut accumulator = String::new();
        loop {
            let mut line = String::new();
            match handle.read_line(&mut line) {
                Ok(0) => return None,             // EOF
                Err(_) => return None,
                Ok(_) => {
                    // Normalise line endings.
                    while line.ends_with('\n') || line.ends_with('\r') {
                        line.pop();
                    }
                    if let Some((body, _)) = find_code_terminator(line.as_bytes()) {
                        let body_str = std::str::from_utf8(body).ok()?;
                        accumulator.push_str(body_str);
                        return Some((accumulator, 0));
                    }
                    accumulator.push_str(&line);
                    accumulator.push('\n');
                }
            }
        }
    }
}

/// Advance the Io::Buffered in_cursor by `n` bytes. Only meaningful in
/// Buffered mode; no-op in Live.
fn advance_io_cursor(n: usize) {
    CURRENT_IO.with(|cell| {
        let mut borrow = cell.borrow_mut();
        if let Some(Io::Buffered { in_cursor, .. }) = borrow.as_mut() {
            *in_cursor += n;
        }
    });
}

fn find_code_terminator(src: &[u8]) -> Option<(&[u8], usize)> {
    const TAG: &[u8] = b";CODE";
    let mut i = 0;
    while i + TAG.len() <= src.len() {
        if &src[i..i + TAG.len()] == TAG {
            let prev_ok = i == 0 || src[i - 1].is_ascii_whitespace();
            let next_ok = i + TAG.len() == src.len() || src[i + TAG.len()].is_ascii_whitespace();
            if prev_ok && next_ok {
                // Consume the trailing newline (and any preceding CR), so
                // the next refill picks up at the START of the line after
                // ;CODE instead of seeing an empty line.
                let mut consumed = i + TAG.len();
                if consumed < src.len() && src[consumed] == b'\r' { consumed += 1; }
                if consumed < src.len() && src[consumed] == b'\n' { consumed += 1; }
                return Some((&src[..i], consumed));
            }
        }
        i += 1;
    }
    None
}

fn with_code_assembler<R>(
    f: impl FnOnce(&mut wfasm::Assembler) -> Result<R, String>,
) -> Result<R, String> {
    CODE_ASSEMBLER.with(|cell| {
        let mut borrowed = cell.borrow_mut();
        if borrowed.is_none() {
            let mut asm = wfasm::Assembler::new();
            asm.register_macro("stk", wfasm::asm::macros::stk);
            // Preload kernel macros (proc, endp, next, pushd, popd, stk,
            // win64_call, brk, plus the @assigns for cell / user-area
            // offsets / tfa constants).
            asm.assemble("macros.masm", MACROS_SOURCE)
                .map_err(|e| format!("preload macros.masm: {e}"))?;
            *borrowed = Some(asm);
        }
        f(borrowed.as_mut().unwrap())
    })
}

/// Write to current output. Buffered: append to vec. Live: stdout + flush.
fn write_bytes(bytes: &[u8]) {
    with_current_io(|io| match io {
        Io::Buffered { output, .. } => output.extend_from_slice(bytes),
        Io::Live { .. } => {
            let mut out = std::io::stdout();
            let _ = out.write_all(bytes);
            let _ = out.flush();
        }
    });
}
