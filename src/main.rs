//! Interactive WF64 REPL.
//!
//! All the heavy lifting lives in `wf64::Wf64Session`. This binary is
//! just the live-stdio shim around it: build a session, hook it to
//! stdin/stdout, run quit until the user types `bye` or stdin hits EOF.

use anyhow::Result;
use std::fs;
use std::path::Path;
use wf64::Wf64Session;

fn main() -> Result<()> {
    let mut session = Wf64Session::new()?;
    let startup = Path::new("lib").join("core.f");
    if startup.is_file() {
        let source = fs::read_to_string(&startup)?;
        let output = session.eval(&source)?;
        if !output.is_empty() {
            print!("{output}");
        }
    }
    session.run_interactive()?;
    Ok(())
}
