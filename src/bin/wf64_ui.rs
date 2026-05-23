//! `wf64-ui` — the WF64 Forth IDE front-end.
//!
//! Phase 2b: spawns a worker thread that owns a `Wf64Session`,
//! loads `lib/core.f`, and drains `IGuiEvent::EvalBuffer` events
//! from the iGui mailbox.  Each event's captured stdout is pushed
//! to the log overlay, followed by a ` ok` line — the standard
//! Forth REPL prompt convention.
//!
//! F5 in the editor pane sends the buffer's text as an
//! EvalBuffer event; this worker is what services it.
//!
//! Single-symbol convention: `∴` (U+2234, "therefore") prefixes
//! the frame and child titles.  Three dots stacked vertically —
//! visually a Forth data stack, mathematically the "therefore"
//! glyph that postfix proof-style reasoning earns.
//!
//! Run with:
//!   cargo run --bin wf64-ui

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Worker thread: owns the Forth session, drains EvalBuffer
    // events, writes captured output to the log overlay.
    let worker = || {
        wait_for_frame();
        auto_open_console();
        run_forth_worker();
    };
    let exit_code = wf64::igui::run(Some(worker))?;
    std::process::exit(exit_code);
}

/// Block until the frame HWND is published.  The frame is created
/// after the worker is spawned, so the worker has to wait before it
/// can post WM_COMMAND messages to it.
#[cfg(windows)]
fn wait_for_frame() {
    use std::time::Duration;
    for _ in 0..200 {  // up to 4s
        if wf64::igui::cp_exports::FRAME_HWND.get().is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    eprintln!("[wf64-ui] FRAME_HWND not published after 4 s; continuing anyway");
}

/// Post WM_COMMAND so the console pane opens on startup.  fedit
/// stays closed — open it via Ctrl+Shift+E when you want it.  The
/// log overlay (Ctrl+Shift+L) is also opt-in.
#[cfg(windows)]
fn auto_open_console() {
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_COMMAND};
    let Some(&hwnd_isize) = wf64::igui::cp_exports::FRAME_HWND.get() else {
        return;
    };
    let hwnd = HWND(hwnd_isize as *mut _);
    let cmd_id = wf64::igui::fconsole::MENU_CMD_ID;
    let _ = unsafe {
        PostMessageW(
            Some(hwnd),
            WM_COMMAND,
            WPARAM(cmd_id as usize),
            LPARAM(0),
        )
    };
}

/// Boot a `Wf64Session`, load core.f, then loop draining events.
/// Restart by dropping the session and bringing a fresh one up
/// when `IGuiEvent::ForthRestart` arrives — invoked from the
/// Forth → Restart menu (Ctrl+Shift+F5).
#[cfg(windows)]
fn run_forth_worker() {
    use wf64::igui::channels::{self, IGuiEvent};
    use wf64::igui::fconsole;

    let mut session = match boot_session(true /* fresh banner */) {
        Some(s) => s,
        None => return,
    };

    loop {
        let twait = std::time::Instant::now();
        let Some(ev) = channels::next_event(200) else {
            continue;
        };
        wf64::igui::fconsole::trace(
            "worker",
            format_args!("recv after {}us", twait.elapsed().as_micros()),
        );
        match ev {
            IGuiEvent::EvalBuffer { source } => {
                let teval = std::time::Instant::now();
                handle_eval(&mut session, &source);
                wf64::igui::fconsole::trace(
                    "worker",
                    format_args!("eval ({} bytes) took {}us",
                        source.len(), teval.elapsed().as_micros()),
                );
            }
            IGuiEvent::ForthRestart => {
                fconsole::reset_for_restart();
                drop(session);
                fconsole::append("∴ restart requested — fresh session below.");
                fconsole::append("");
                match boot_session(false) {
                    Some(s) => session = s,
                    None => return,
                }
            }
            IGuiEvent::FrameClose => {
                fconsole::append("∴ frame closing");
                break;
            }
            _ => {}
        }
    }
}

/// Create a session, load core.f, emit the startup banner.  Used
/// both for the initial boot and for Forth → Restart.  `intro`
/// controls whether the welcome lines print (we skip them after
/// a restart since the console already shows a "restart requested"
/// notice).
#[cfg(windows)]
fn boot_session(intro: bool) -> Option<wf64::Wf64Session> {
    use std::path::Path;
    use wf64::igui::fconsole;

    if intro {
        fconsole::append("∴ WF64 — Forth IDE");
        fconsole::append("");
        fconsole::append("Type at the prompt, press Enter.");
        fconsole::append("Editor: Ctrl+Shift+E   Console: Ctrl+Shift+R   Restart: Ctrl+Shift+F5");
        fconsole::append("");
    }

    let mut session = match wf64::Wf64Session::new() {
        Ok(s) => s,
        Err(e) => {
            fconsole::append(&format!("∴ session boot failed: {e}"));
            return None;
        }
    };

    let core_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("lib").join("core.f");
    match session.load_source_file(&core_path) {
        Ok(()) => fconsole::append(&format!("∴ loaded {}", core_path.display())),
        Err(e) => fconsole::append(&format!("∴ core.f load failed: {e}")),
    }
    // No manual " ok" — the include itself emits one through the
    // captured IO stream when it succeeds.
    fconsole::append("");
    Some(session)
}

/// Run one source chunk through the session and pipe the result
/// to the console.  Single-line submissions get the plain
/// "<output> ok" treatment a real REPL does; multi-line buffers
/// (F5 from fedit) get a header / footer so the transcript is
/// scannable.
#[cfg(windows)]
fn handle_eval(session: &mut wf64::Wf64Session, source: &str) {
    use wf64::igui::fconsole;
    let multiline = source.lines().count() > 1;
    if multiline {
        fconsole::append("─── eval ───");
        for line in source.lines().take(8) {
            fconsole::append(line);
        }
        let extra = source.lines().count().saturating_sub(8);
        if extra > 0 {
            fconsole::append(&format!("    … {extra} more line(s) elided"));
        }
        fconsole::append("─── result ───");
    }
    match session.eval(source) {
        Ok(output) => {
            // The Forth interpreter itself emits ` ok\n` at the end
            // of a successful eval — DON'T add another one here, or
            // you'll see "ok ok" on every prompt.
            let trimmed = output.trim_end_matches('\n');
            if !trimmed.is_empty() {
                for line in trimmed.lines() {
                    fconsole::append(line);
                }
            }
        }
        Err(e) => {
            fconsole::append(&format!("{e}"));
        }
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("wf64-ui is Windows-only (iGui depends on Direct2D / DirectWrite).");
    std::process::exit(1);
}
