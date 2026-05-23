//! `wf64-ui` — the WF64 Forth IDE front-end.
//!
//! Phase 1 scaffolding: brings up an MDI frame using the ported
//! iGui module (originally NewCormanLisp's `iGui`).  No Forth
//! code runs yet — the goal here is to confirm the Direct2D /
//! DirectWrite renderer compiles and a window opens cleanly on
//! this machine.  Phase 2 adds an editor pane plus an `F5` →
//! `Wf64Session::eval` bridge so saved buffers can be run
//! through the live Forth.
//!
//! Run with:
//!   cargo run --bin wf64-ui

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // No worker thread yet — Phase 2 will spawn the Forth REPL here.
    let exit_code = wf64::igui::run::<fn()>(None)?;
    std::process::exit(exit_code);
}

#[cfg(not(windows))]
fn main() {
    eprintln!("wf64-ui is Windows-only (iGui depends on Direct2D / DirectWrite).");
    std::process::exit(1);
}
