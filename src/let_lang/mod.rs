//! LET — a small infix-algebraic DSL embedded in Forth.
//!
//! `LET ( in1, in2, ... ) -> ( out1, out2, ... ) = expr_list WHERE ... END`
//!
//! Compiles to a stand-alone Win64 function `(inputs*, outputs*) -> ()`
//! whose body lives in a fresh JIT module.  The Forth `LET` immediate
//! word reads source up to `END`, calls [`compile`], stores the resulting
//! function in the session so its code stays mapped, and emits a small
//! trampoline at HERE that loads the inputs from the Forth FP stack,
//! calls the compiled function, and pushes the outputs back.
//!
//! Scope of the MVP:
//! * Operators: `+ - * /` and unary `-`.
//! * Constants: `pi`, `e`, plus any numeric literals.
//! * WHERE bindings with dependency-ordered eval and cycle detection.
//! * Multiple inputs and multiple outputs.
//!
//! Not yet implemented (future work):
//! * Function calls (`sin`, `cos`, `sqrt`, `pow`, `hypot`, ...).
//! * `**` operator (needs `pow`).
//! * `select(cond, a, b)` / `IF/THEN/ELSE`.

pub mod parser;
pub mod codegen;

pub use parser::{LetError, LetForm};

/// Result of compiling one LET form to MC-flavour Intel asm text.
#[derive(Debug)]
pub struct CompiledLet {
    /// Function name as emitted into the asm. Must be unique per JIT module.
    pub fn_name: String,
    /// Asm source text ready for `Jit::add_asm`.
    pub asm_text: String,
    /// Number of inputs the function reads from `[rcx + i*8]`.
    pub n_inputs: usize,
    /// Number of outputs the function writes to `[rdx + i*8]` (with
    /// `outputs[0]` being the rightmost result = FP stack TOS).
    pub n_outputs: usize,
}

/// Parse and lower a LET form. Does not JIT-compile.
pub fn compile(source: &str, fn_name: &str) -> Result<CompiledLet, LetError> {
    let form = parser::parse(source)?;
    let asm_text = codegen::lower(&form, fn_name)?;
    Ok(CompiledLet {
        fn_name: fn_name.to_string(),
        asm_text,
        n_inputs: form.inputs.len(),
        n_outputs: form.outputs.len(),
    })
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use wfasm::Jit;

    /// Take a LET source, compile it, JIT-load it, then call it as a
    /// Win64 fn(*const f64, *mut f64) -> () and check the outputs.
    ///
    /// `inputs` and `expected` are in **Forth FP-stack order**: index 0
    /// is TOS (lowest address), index N-1 is the deepest cell. So if the
    /// LET signature is `(a, b, c) -> (...)`, the user pushes a, then b,
    /// then c — at call time the stack reads `[c, b, a]` and our test
    /// passes `&[c, b, a]`.
    fn run_let(source: &str, fn_name: &str, inputs: &[f64], expected: &[f64]) {
        let compiled = compile(source, fn_name)
            .unwrap_or_else(|e| panic!("compile failed: {e}\nsource: {source}"));
        let mut jit = Jit::new(&format!("let_test_{fn_name}")).expect("Jit::new");
        jit.add_asm(&compiled.asm_text)
            .unwrap_or_else(|e| panic!("add_asm failed: {e:?}\nasm:\n{}", compiled.asm_text));
        // Declare the symbol in IR so MCJIT keeps it after link.
        jit.declare_fn(fn_name, 0).expect("declare_fn");
        let addr = jit
            .lookup_addr(fn_name)
            .unwrap_or_else(|e| panic!("lookup_addr failed: {e:?}\nasm:\n{}", compiled.asm_text));

        // Win64: rcx = inputs, rdx = outputs.
        let f: unsafe extern "system" fn(*const f64, *mut f64) = unsafe { std::mem::transmute(addr) };
        let mut outputs = vec![0.0_f64; compiled.n_outputs];
        unsafe { f(inputs.as_ptr(), outputs.as_mut_ptr()); }

        for (i, (got, want)) in outputs.iter().zip(expected.iter()).enumerate() {
            let diff = (got - want).abs();
            assert!(
                diff < 1e-9,
                "output[{i}]: got {got}, expected {want} (diff {diff})\nasm:\n{}",
                compiled.asm_text,
            );
        }
        // Keep the Jit alive until after we're done with the fn pointer.
        drop(jit);
    }

    #[test]
    fn jit_compiles_identity() {
        // Outputs convention: outputs[0] is the rightmost result.
        run_let("LET (x) -> (y) = x END", "let_id", &[42.0], &[42.0]);
    }

    #[test]
    fn jit_compiles_arithmetic() {
        run_let(
            "LET (x) -> (y) = x * x + 1 END",
            "let_quad",
            &[5.0],
            &[26.0],
        );
    }

    #[test]
    fn jit_compiles_area_of_circle() {
        run_let(
            "LET (r) -> (a) = pi * r * r END",
            "let_area_ic",
            &[2.0],
            &[std::f64::consts::PI * 4.0],
        );
    }

    #[test]
    fn jit_compiles_multi_input_multi_output() {
        // Forth: `10. 3. addsub`  pushes a=10 first then b=3, so memory
        // reads [b=3, a=10] = inputs `&[3.0, 10.0]` in our convention.
        // Outputs: declared (diff, sum); sum is last-declared so it ends
        // up at TOS → outputs `&[sum, diff]` = `&[13.0, 7.0]`.
        run_let(
            "LET (a, b) -> (diff, sum) = a - b, a + b END",
            "let_addsub",
            &[3.0, 10.0],     // [b=TOS, a=NOS]
            &[13.0, 7.0],     // [sum=TOS, diff=NOS]
        );
    }

    #[test]
    fn jit_compiles_mbrot_step() {
        // Forth call: `1. 1. 1. 1. mbrot`. Inputs in memory: [y, x, z_im, z_re].
        // Outputs declared (z_next_re, z_next_im, mag); mag is TOS.
        run_let(
            "LET (z_re, z_im, x, y) -> (z_next_re, z_next_im, mag) = \
                re, im, rmag \
                WHERE re   = z_re * z_re - z_im * z_im + x \
                WHERE im   = 2 * z_re * z_im + y \
                WHERE rmag = re * re + im * im \
             END",
            "let_mbrot",
            &[1.0, 1.0, 1.0, 1.0],
            // re   = 1 - 1 + 1 = 1
            // im   = 2 * 1 * 1 + 1 = 3
            // rmag = 1 + 9 = 10
            &[10.0, 3.0, 1.0],   // [mag, im, re]
        );
    }

    #[test]
    fn jit_compiles_unary_minus() {
        run_let("LET (x) -> (y) = -x END", "let_neg", &[7.5], &[-7.5]);
    }

    #[test]
    fn jit_compiles_negative_zero_handles_correctly() {
        // -0.0 is its own bit pattern; the sign-mask XOR should flip it.
        run_let("LET (x) -> (y) = -x END", "let_neg0", &[0.0], &[-0.0]);
    }

    #[test]
    fn jit_compiles_division() {
        // Forth: `100. 8. div`  → b=8 at TOS, a=100 at NOS.
        // memory order (TOS first): [b=8, a=100].
        run_let("LET (a, b) -> (q) = a / b END", "let_div", &[8.0, 100.0], &[12.5]);
    }
}

