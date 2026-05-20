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
        output: Vec<u8>,
    },
    /// Real stdin/stdout — for the interactive REPL.
    Live,
}

impl Io {
    pub fn new_buffered() -> Self {
        Io::Buffered { input: Vec::new(), in_cursor: 0, output: Vec::new() }
    }
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
        Io::Live => {
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

/// Write to current output. Buffered: append to vec. Live: stdout + flush.
fn write_bytes(bytes: &[u8]) {
    with_current_io(|io| match io {
        Io::Buffered { output, .. } => output.extend_from_slice(bytes),
        Io::Live => {
            let mut out = std::io::stdout();
            let _ = out.write_all(bytes);
            let _ = out.flush();
        }
    });
}
