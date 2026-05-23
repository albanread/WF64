//! In-process Factor VM session.
//!
//! ## Architecture
//!
//! ```text
//! Worker thread (owns FactorSession)
//! │   transpiler: Forth → Factor source
//! │
//! │  stdin_write ──────────► Factor listener thread
//! │                               (start_standalone_factor_in_new_thread)
//! │  stdout_read ◄──────────       writes output to our pipe
//! ```
//!
//! Factor runs in a **dedicated OS thread within our process** — no
//! subprocess is spawned.  The Factor thread is started by calling
//! `start_standalone_factor_in_new_thread` from `factor.dll` (a plain
//! C export, accessed via `libloading`).
//!
//! ## I/O Redirection
//!
//! Before starting the Factor thread we redirect CRT file descriptors
//! 0 (stdin) and 1 (stdout) to a pair of anonymous Windows pipes.
//! Factor's `init_factor` captures the CRT `stdin`/`stdout` FILE*
//! objects via its `VALID_HANDLE` macro, which uses fd 0/1 — so Factor
//! automatically reads from our write-end and writes to our read-end.
//!
//! Because `newfactor-ui` is a GUI application the process has no
//! console; CRT stdin/stdout are unattached (`/dev/nul`).  Redirecting
//! them to our pipes is safe and does not break any other I/O.
//!
//! ## Listener Suppression
//!
//! Factor's `listener-step` normally prints the data-stack and a vocab
//! prompt (`IN: scratchpad`) at the START of every step (before reading).
//! During bootstrap we suppress both:
//!
//! ```factor
//! display-stacks? off           ! stop printing the data-stack between steps
//! M: object prompt. 2drop ;    ! redefine prompt method to do nothing
//! ```
//!
//! After that the pipe contains only output explicitly produced by user code.
//!
//! ## Eval Protocol
//!
//! For each evaluation we send TWO lines:
//!
//! ```factor
//! <transpiled-factor-code>
//! "%%NF-DONE%%\n" write flush
//! ```
//!
//! Line 1 is the user expression.  If it throws, Factor's `call-error-hook`
//! prints a formatted error and `recover` keeps the listener alive.  Either
//! way, Factor then reads Line 2, writes our sentinel to stdout, and the
//! `read_until_sentinel` call returns with everything up to (not including)
//! the sentinel line.
//!
//! ## Stack Query
//!
//! After eval, we query the data stack:
//! ```factor
//! get-datastack [ dup integer? [ . ] [ drop ] if ] each
//! "%%NF-STACK%%\n" write flush
//! ```
//! Stack items are printed one per line (integers only).

use std::io::{self, BufRead, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::newfactor::transpiler::Transpiler;

// ── Configuration ─────────────────────────────────────────────────────────

/// Root of the NewFactor repository — vocabularies, image, and DLL all live here.
pub const NEWFACTOR_ROOT: &str = "E:\\NewFactor";

/// Path to the Factor DLL (copied into the NewFactor repo).
pub const FACTOR_DLL: &str = "E:\\NewFactor\\factor.dll";
/// Path to the Factor boot image (copied into the NewFactor repo).
pub const FACTOR_IMAGE: &str = "E:\\NewFactor\\factor.image";

/// Sentinel written by Factor after each eval (or error recovery).
const SENTINEL_DONE:  &str = "%%NF-DONE%%";
/// Sentinel used to probe for Factor readiness.
const SENTINEL_READY: &str = "%%NF-READY%%";
/// Sentinel that ends a stack dump.
const SENTINEL_STACK: &str = "%%NF-STACK%%";

/// Time to wait for Factor's image to load before probing.
const STARTUP_WAIT_MS: u64 = 4_000;
/// Maximum time to wait for any sentinel response.
const SENTINEL_TIMEOUT: Duration = Duration::from_secs(30);

// ── Win32 / CRT raw FFI ───────────────────────────────────────────────────
//
// We use raw extern declarations so we don't pull in extra windows-crate
// features.  All of these are always present on Windows (kernel32.dll /
// ucrtbase.dll).

type HANDLE = *mut std::ffi::c_void;
const NULL: HANDLE = std::ptr::null_mut();

#[link(name = "kernel32")]
extern "system" {
    fn CreatePipe(
        hReadPipe: *mut HANDLE,
        hWritePipe: *mut HANDLE,
        lpPipeAttributes: *const std::ffi::c_void, // SECURITY_ATTRIBUTES*, may be null
        nSize: u32,
    ) -> i32; // BOOL: 0 = failure

    /// Add a directory to the beginning of the DLL search path.
    /// Called before loading factor.dll so that its sibling DLLs
    /// (libssl, libcrypto, sqlite3) are found automatically.
    fn SetDllDirectoryW(lpPathName: *const u16) -> i32;
}

extern "C" {
    /// Associate a Win32 HANDLE with a new CRT file descriptor.
    /// The CRT takes ownership of the handle; do NOT close it separately.
    /// Returns the new fd (≥ 0) on success, -1 on error.
    fn _open_osfhandle(osfhandle: isize, flags: i32) -> i32;

    /// Duplicate CRT fd `fd1` onto fd slot `fd2` (POSIX dup2).
    /// Returns 0 on success, -1 on error.
    fn _dup2(fd1: i32, fd2: i32) -> i32;

    /// Close a CRT file descriptor and its underlying HANDLE.
    fn _close(fd: i32) -> i32;
}

// ── Helpers ───────────────────────────────────────────────────────────────

/// Create an anonymous Windows pipe, returning `(read_end, write_end)`.
fn create_pipe(buf_size: u32) -> Result<(HANDLE, HANDLE)> {
    let mut r: HANDLE = NULL;
    let mut w: HANDLE = NULL;
    let ok = unsafe { CreatePipe(&mut r, &mut w, NULL, buf_size) };
    anyhow::ensure!(ok != 0, "CreatePipe failed");
    Ok((r, w))
}

/// Convert a Win32 HANDLE to a Rust `File` by wrapping it.
///
/// # Safety
/// `handle` must be a valid, open handle that this `File` will own.
unsafe fn handle_to_file(handle: HANDLE) -> std::fs::File {
    use std::os::windows::io::FromRawHandle;
    std::fs::File::from_raw_handle(handle)
}

// ── FactorSession ──────────────────────────────────────────────────────────

/// An in-process Factor VM session.
///
/// Owns the write end of Factor's stdin pipe and the read end of
/// Factor's stdout pipe.  All eval calls are synchronous: we write
/// Factor code to the pipe and block until we see the sentinel line.
pub struct FactorSession {
    transpiler: Transpiler,
    /// BufWriter around the write end of Factor's stdin pipe.
    stdin_writer: io::BufWriter<std::fs::File>,
    /// BufReader around the read end of Factor's stdout pipe.
    stdout_reader: io::BufReader<std::fs::File>,
    /// Last known data stack (integers only).
    data_stack: Vec<i64>,
}

// SAFETY: FactorSession holds std::fs::File objects (Send) and a
// Transpiler (plain Rust, Send).  The Factor thread owns its own TLS
// and VM state; we communicate only through the pipes.
unsafe impl Send for FactorSession {}

impl FactorSession {
    // ── Construction ───────────────────────────────────────────────────

    /// Create a new Factor session.
    ///
    /// 1. Creates anonymous pipe pairs for stdin and stdout.
    /// 2. Redirects CRT fd 0/1 to the pipes so Factor sees them.
    /// 3. Starts `factor.dll`'s listener thread.
    /// 4. Waits for Factor to be ready, then loads `forth.all`.
    pub fn new() -> Result<Self> {
        eprintln!("[NF] FactorSession::new — creating pipes");
        // stdin pipe: Factor reads from stdin_r; we write to stdin_w.
        let (stdin_r, stdin_w) = create_pipe(65_536)
            .context("CreatePipe (Factor stdin)")?;

        // stdout pipe: we read from stdout_r; Factor writes to stdout_w.
        let (stdout_r, stdout_w) = create_pipe(256_000)
            .context("CreatePipe (Factor stdout)")?;

        eprintln!("[NF] pipes created — redirecting CRT fds");
        // ── Redirect CRT fds 0 and 1 ──────────────────────────────────
        //
        // _open_osfhandle transfers handle ownership to the CRT.
        // _dup2 installs a copy on fd 0 or 1.
        // _close releases the original temporary fd (0/1 remain open).
        unsafe {
            let r_fd = _open_osfhandle(stdin_r as isize, 0 /* O_RDONLY */);
            anyhow::ensure!(r_fd >= 0, "_open_osfhandle(stdin_r) failed");
            let rc = _dup2(r_fd, 0); // redirect CRT stdin → our pipe
            _close(r_fd);
            anyhow::ensure!(rc == 0, "_dup2(stdin → 0) failed");

            let w_fd = _open_osfhandle(stdout_w as isize, 0 /* O_WRONLY */);
            anyhow::ensure!(w_fd >= 0, "_open_osfhandle(stdout_w) failed");
            let rc = _dup2(w_fd, 1); // redirect CRT stdout → our pipe
            _close(w_fd);
            anyhow::ensure!(rc == 0, "_dup2(stdout → 1) failed");
        }

        eprintln!("[NF] fd redirect done — loading factor.dll");
        // ── Start Factor listener thread ──────────────────────────────

        Self::start_factor_thread().context("start Factor listener thread")?;
        eprintln!("[NF] factor.dll loaded, thread started");

        // ── Wrap remaining pipe ends as Rust Files ─────────────────────
        //
        // stdin_r and stdout_w are now owned by CRT fds 0 and 1.
        // We keep stdin_w (write to Factor) and stdout_r (read from Factor).
        let stdin_file  = unsafe { handle_to_file(stdin_w)  };
        let stdout_file = unsafe { handle_to_file(stdout_r) };

        let mut session = FactorSession {
            transpiler:    Transpiler::new(),
            stdin_writer:  io::BufWriter::new(stdin_file),
            stdout_reader: io::BufReader::new(stdout_file),
            data_stack:    Vec::new(),
        };

        // ── Wait for Factor, then bootstrap ──────────────────────────

        eprintln!("[NF] sleeping {}ms for image load", STARTUP_WAIT_MS);
        session.wait_for_ready().context("Factor startup probe")?;
        eprintln!("[NF] Factor ready — bootstrapping");
        session.bootstrap().context("forth.all bootstrap")?;
        eprintln!("[NF] bootstrap done");

        Ok(session)
    }

    /// Load `factor.dll` via libloading and call
    /// `start_standalone_factor_in_new_thread`.
    ///
    /// Factor's listener starts in a new OS thread within our process.
    /// It reads from CRT fd 0 (our stdin pipe) and writes to fd 1
    /// (our stdout pipe) because we redirected them above.
    fn start_factor_thread() -> Result<()> {
        // factor.dll exports this as a C symbol (VM_C_API = dllexport + extern "C").
        type StartFactorFn = unsafe extern "C" fn(
            argc: i32,
            argv: *mut *mut u16, // wchar_t** on Windows
        ) -> *mut std::ffi::c_void; // returns THREADHANDLE (HANDLE)

        // Prepend the NewFactor repo directory to the DLL search path so
        // that factor.dll's runtime dependencies (libssl, libcrypto,
        // sqlite3) are found even when the binary lives elsewhere.
        let dir_wide: Vec<u16> = NEWFACTOR_ROOT
            .encode_utf16()
            .chain(Some(0u16))
            .collect();
        unsafe { SetDllDirectoryW(dir_wide.as_ptr()); }

        let lib = unsafe {
            libloading::Library::new(FACTOR_DLL)
                .with_context(|| format!("load {FACTOR_DLL}"))?
        };

        let start_fn: libloading::Symbol<StartFactorFn> = unsafe {
            lib.get(b"start_standalone_factor_in_new_thread\0")
                .context("find start_standalone_factor_in_new_thread in factor.dll")?
        };

        // Build wide-string argv.
        // Factor's init_from_args parses -i= (image) and -no-signals.
        let args_utf8: &[&str] = &[
            "newfactor-ui.exe",          // argv[0] — Factor uses this for crash dumps
            &format!("-i={FACTOR_IMAGE}"),
            "-no-signals",               // don't install signal handlers (conflicts with Rust's)
        ];
        let wide: Vec<Vec<u16>> = args_utf8.iter()
            .map(|s| s.encode_utf16().chain(Some(0u16)).collect())
            .collect();
        let mut ptrs: Vec<*mut u16> = wide.iter()
            .map(|v| v.as_ptr() as *mut u16)
            .collect();

        // `ptrs` and `wide` live until the end of this function, which is
        // past the call — Factor only needs argv during `init_from_args`.
        let _thread_handle = unsafe {
            start_fn(ptrs.len() as i32, ptrs.as_mut_ptr())
        };

        // Intentionally leak the Library: the DLL must stay loaded for the
        // lifetime of the process (the Factor thread is still running in it).
        std::mem::forget(lib);

        Ok(())
    }

    // ── Startup / bootstrap ────────────────────────────────────────────

    /// Wait for Factor to finish loading its image, then verify it's alive.
    ///
    /// We sleep first so that Factor's Stage 2 compilation output is already
    /// buffered in the pipe before we start our sentinel handshake.  The
    /// listener may still be mid-startup when the probe arrives; it will
    /// process the probe as soon as it enters its first `listener-step`.
    fn wait_for_ready(&mut self) -> Result<()> {
        // Factor's Stage 2 compilation takes a few seconds on first boot.
        std::thread::sleep(Duration::from_millis(STARTUP_WAIT_MS));

        // Ask Factor to print our ready sentinel.
        self.send_raw("\"%%NF-READY%%\\n\" write flush\n")
            .context("send ready probe")?;

        // Drain Factor's startup banner until we see the sentinel.
        self.read_until_sentinel(SENTINEL_READY, SENTINEL_TIMEOUT)
            .context("wait for Factor ready sentinel")?;

        Ok(())
    }

    /// Load the NewFactor vocabulary suite (`forth.all`) into Factor,
    /// then silence the listener's verbose output.
    fn bootstrap(&mut self) -> Result<()> {
        // Escape backslashes for Factor string literals.
        let root_escaped = NEWFACTOR_ROOT.replace('\\', "\\\\");

        // Each line is one `listener-step`.  The listener reads them from the
        // pipe one by one and executes them.
        //
        // After loading forth.all, we suppress the two sources of noise that
        // the listener normally injects between evals:
        //
        //   display-stacks? off
        //       Factor's listener-step prints the data-stack at the START of
        //       every step (before reading the next expression).  Turning this
        //       off stops the `3\n5\n` etc. appearing in our output stream.
        //
        //   M: object prompt. 2drop ;
        //       listener-step also prints the vocab prompt ("IN: scratchpad\n")
        //       by calling `prompt.` on the current input stream.  Redefining
        //       the catch-all method to do nothing silences it permanently.
        //
        // After these two lines take effect, our pipe is clean: only text that
        // user code explicitly writes appears between successive sentinels.
        let boot = format!(concat!(
            "USE: vocabs.loader\n",
            "\"{root}\" add-vocab-root\n",
            "USE: forth.all\n",
            "display-stacks? off\n",
            "M: object prompt. 2drop ;\n",
            "\"%%NF-READY%%\\n\" write flush\n",
        ), root = root_escaped);

        self.send_raw(&boot).context("send bootstrap")?;

        self.read_until_sentinel(SENTINEL_READY, SENTINEL_TIMEOUT)
            .context("wait for bootstrap ready sentinel")?;

        Ok(())
    }

    // ── Low-level I/O ─────────────────────────────────────────────────

    fn send_raw(&mut self, code: &str) -> Result<()> {
        self.stdin_writer.write_all(code.as_bytes())
            .context("write to Factor stdin pipe")?;
        self.stdin_writer.flush()
            .context("flush Factor stdin pipe")?;
        Ok(())
    }

    /// Read from Factor's stdout until a line consists solely of `sentinel`.
    ///
    /// Returns all output lines that appeared before the sentinel.
    /// Returns an error if `timeout` elapses.
    fn read_until_sentinel(&mut self, sentinel: &str, timeout: Duration) -> Result<String> {
        let deadline = Instant::now() + timeout;
        let mut output = String::new();
        let mut line = String::new();

        loop {
            if Instant::now() > deadline {
                anyhow::bail!(
                    "timed out ({timeout:?}) waiting for Factor sentinel {sentinel:?}"
                );
            }
            line.clear();
            let n = self.stdout_reader.read_line(&mut line)
                .context("read from Factor stdout pipe")?;
            if n == 0 {
                anyhow::bail!("Factor stdout EOF while waiting for sentinel {sentinel:?}");
            }
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed == sentinel {
                return Ok(output);
            }
            output.push_str(&line);
        }
    }

    // ── Public API ────────────────────────────────────────────────────

    /// Evaluate Forth source code.
    ///
    /// The code is transpiled to Factor, sent to the listener, and the
    /// output is returned.  On error Factor's `call-error-hook` prints a
    /// formatted error and `recover` keeps the listener alive; the error
    /// text appears in the returned string before the sentinel.
    pub fn eval(&mut self, forth_source: &str) -> Result<String> {
        let factor_code = self.transpiler.transpile(forth_source);

        // Two listener-steps:
        //   Step 1 — user's transpiled Factor code (may produce output / errors)
        //   Step 2 — write the done-sentinel and flush; always executes because
        //             Factor's listener recovers from any error in step 1.
        //
        // The listener prints NO ` ok` nor any prompt between steps (those were
        // suppressed during bootstrap), so captured output is exactly what the
        // user code writes to stdout.
        let msg = format!(
            "{factor_code}\n\
             \"{SENTINEL_DONE}\\n\" write flush\n"
        );

        self.send_raw(&msg).context("send eval to Factor")?;

        self.read_until_sentinel(SENTINEL_DONE, SENTINEL_TIMEOUT)
            .context("receive eval output from Factor")
    }

    /// Query the current Factor data stack.
    ///
    /// Returns stack contents (integers only) with TOS last.
    pub fn stack(&mut self) -> Vec<i64> {
        let probe = format!(
            "get-datastack \
             [ dup integer? [ . ] [ drop ] if ] each\n\
             \"{SENTINEL_STACK}\\n\" write flush\n"
        );
        if self.send_raw(&probe).is_err() {
            return self.data_stack.clone();
        }
        match self.read_until_sentinel(SENTINEL_STACK, Duration::from_secs(5)) {
            Ok(raw) => {
                let parsed: Vec<i64> = raw.lines()
                    .filter_map(|l| l.trim().parse().ok())
                    .collect();
                self.data_stack = parsed;
            }
            Err(e) => eprintln!("[FactorSession::stack] {e}"),
        }
        self.data_stack.clone()
    }

    /// Reset the transpiler ctrl-stack.
    /// Does NOT restart the Factor VM.
    pub fn reset(&mut self) {
        self.transpiler.reset();
    }

    /// Load a source file.
    /// `.fth` files go through `forth-load`; `.factor` files use `load-file`.
    pub fn load_source_file(&mut self, path: &Path) -> Result<()> {
        let path_str = path.to_string_lossy();
        let escaped  = path_str.replace('\\', "\\\\");

        let code = if path.extension().and_then(|e| e.to_str()) == Some("fth") {
            format!("\"{escaped}\" forth-load")
        } else {
            format!("\"{escaped}\" load-file")
        };

        self.eval(&code)
            .with_context(|| format!("load {}", path.display()))?;
        Ok(())
    }
}
