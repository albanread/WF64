//! LET codegen: AST → raw MC-flavour Intel asm text.
//!
//! Calling convention (Win64):
//!   rcx = inputs pointer  — the i-th DECLARED input is read from
//!         [rcx + (n_in - 1 - i) * 8].  This matches Forth FP-stack
//!         layout: the last-declared input is TOS (lowest address),
//!         the first-declared input is the deepest stack cell.
//!   rdx = outputs pointer — the i-th DECLARED output is written to
//!         [rdx + (n_out - 1 - i) * 8].  Again matches Forth: the
//!         last-declared output ends up at TOS after the call.
//!
//! Net effect: a runtime trampoline can pass rcx = FSP (current FP TOS)
//! and rdx = (FSP after the call's net pop/push), and no copy is needed
//! either way — the function reads inputs straight off the FP stack and
//! writes outputs straight onto it.
//!
//! All computation lives in xmm0..xmm15. Each "named" value (input or
//! WHERE binding) gets a fixed register for the duration of the function;
//! sub-expression evaluation uses the remaining xmm regs as scratch.
//!
//! For an expression of depth d, peak register pressure is
//! `(named regs) + d`. With 16 xmm regs and typical LETs having ~3-6
//! named values, we comfortably handle expressions up to ~10 deep.

use std::collections::HashMap;
use super::parser::{LetForm, Expr, BinOp};
use super::LetError;

const TOTAL_XMM: u8 = 16;

pub fn lower(form: &LetForm, fn_name: &str) -> Result<String, LetError> {
    // 1. Resolve every Var reference / Call name.
    for e in &form.results { validate_expr(e, form)?; }
    for (_, e) in &form.wheres { validate_expr(e, form)?; }

    // 2. Topo-sort WHERE bindings so dependencies are emitted before
    //    their dependents.  Cycles are reported as errors.
    let order = topo_sort_wheres(&form.wheres)?;

    // 3. Assign one xmm reg per named value (input + WHERE binding).
    let mut vars: HashMap<String, u8> = HashMap::new();
    let mut next: u8 = 0;
    for input in &form.inputs {
        vars.insert(input.clone(), next);
        next += 1;
    }
    for &i in &order {
        vars.insert(form.wheres[i].0.clone(), next);
        next += 1;
    }
    // Need at least 2 scratch regs for tree eval.  Reject overflow.
    if next + 2 > TOTAL_XMM {
        return Err(LetError {
            message: format!(
                "LET has {} named values; needs ≤ {} for xmm0..xmm{} (this build supports up to {} named values)",
                next, TOTAL_XMM - 2, TOTAL_XMM - 1, TOTAL_XMM - 2,
            ),
            pos: 0,
        });
    }

    // 4. Emit.
    let mut s = String::new();
    let mut const_pool: Vec<u64> = Vec::new();   // raw bits, dedup by bit-equal

    // MC defaults to AT&T syntax on x86_64; switch on Intel.
    s.push_str("    .intel_syntax noprefix\n");
    s.push_str("    .text\n");
    s.push_str(&format!("    .globl {fn_name}\n"));
    s.push_str(&format!("{fn_name}:\n"));

    // Load inputs.  Declared input `i` lives at [rcx + (n_in-1-i)*8]
    // because the Forth FP stack puts the LAST-declared input at TOS
    // (lowest address) and the first-declared input deepest.
    let n_in = form.inputs.len();
    for (i, name) in form.inputs.iter().enumerate() {
        let r = vars[name];
        let off = (n_in - 1 - i) * 8;
        s.push_str(&format!(
            "    movsd xmm{r}, qword ptr [rcx + {off}]\n",
        ));
    }

    // Emit WHERE bindings in dependency order.
    for &i in &order {
        let (name, expr) = &form.wheres[i];
        let target = vars[name];
        emit_expr(expr, target, &vars, &mut const_pool, next, fn_name, &mut s)?;
    }

    // Emit results into the outputs buffer.
    // Declared output `i` is written to [rdx + (n_out - 1 - i)*8], so
    // the last-declared output lands at TOS (lowest address) on the
    // Forth FP stack — symmetric with how inputs are loaded.
    let n_out = form.results.len();
    let scratch_for_result = next;   // first scratch reg
    for (i, expr) in form.results.iter().enumerate() {
        emit_expr(expr, scratch_for_result, &vars, &mut const_pool, next + 1, fn_name, &mut s)?;
        let off = (n_out - 1 - i) * 8;
        s.push_str(&format!(
            "    movsd qword ptr [rdx + {off}], xmm{scratch_for_result}\n",
        ));
    }

    s.push_str("    ret\n");

    // Constant pool (rodata-style, but the assembler emits it into .text
    // alongside the code — which is fine for MCJIT, the bytes are
    // executable-AND-readable in our scheme).
    if !const_pool.is_empty() {
        s.push_str("    .p2align 3\n");
        for (i, bits) in const_pool.iter().enumerate() {
            let f = f64::from_bits(*bits);
            s.push_str(&format!(
                "{fn_name}$$const_{i}: .quad 0x{:016X}    # {f}\n", bits,
            ));
        }
    }
    // Sign mask for unary negate. Always emit even if unused; it's only
    // 16 bytes and keeps codegen simpler.
    s.push_str("    .p2align 4\n");
    s.push_str(&format!(
        "{fn_name}$$sign_mask: .quad 0x8000000000000000, 0x0000000000000000\n",
    ));

    Ok(s)
}

/// Emit code that leaves `expr`'s value in xmm{target}.
/// `next_scratch` is the first xmm reg index available for sub-expression
/// scratch (guaranteed not to alias any named value).
fn emit_expr(
    expr: &Expr,
    target: u8,
    vars: &HashMap<String, u8>,
    const_pool: &mut Vec<u64>,
    next_scratch: u8,
    fn_name: &str,
    out: &mut String,
) -> Result<(), LetError> {
    match expr {
        Expr::Lit(n) => {
            emit_load_const(target, *n, const_pool, fn_name, out);
        }
        Expr::Var(name) => {
            if let Some(c) = known_constant(name) {
                emit_load_const(target, c, const_pool, fn_name, out);
            } else if let Some(&src) = vars.get(name) {
                if src != target {
                    out.push_str(&format!("    movsd xmm{target}, xmm{src}\n"));
                }
            } else {
                return Err(LetError {
                    message: format!("undefined name '{name}'"),
                    pos: 0,
                });
            }
        }
        Expr::Bin(op, l, r) => {
            // LHS into target, RHS into next_scratch, then combine.
            if next_scratch >= TOTAL_XMM {
                return Err(LetError {
                    message: "LET expression too deep for register file".into(),
                    pos: 0,
                });
            }
            emit_expr(l, target, vars, const_pool, next_scratch, fn_name, out)?;
            emit_expr(r, next_scratch, vars, const_pool, next_scratch + 1, fn_name, out)?;
            let m = match op {
                BinOp::Add => "addsd",
                BinOp::Sub => "subsd",
                BinOp::Mul => "mulsd",
                BinOp::Div => "divsd",
                BinOp::Pow => return Err(LetError {
                    message: "** is not supported in the MVP (needs libm pow); use repeated *".into(),
                    pos: 0,
                }),
            };
            out.push_str(&format!("    {m} xmm{target}, xmm{next_scratch}\n"));
        }
        Expr::Neg(e) => {
            emit_expr(e, target, vars, const_pool, next_scratch, fn_name, out)?;
            out.push_str(&format!(
                "    xorpd xmm{target}, xmmword ptr [rip + {fn_name}$$sign_mask]\n",
            ));
        }
        Expr::Call(name, _) => {
            return Err(LetError {
                message: format!("function call '{name}' not yet implemented (MVP supports + - * / unary-minus and named constants pi/e)"),
                pos: 0,
            });
        }
    }
    Ok(())
}

fn emit_load_const(target: u8, value: f64, pool: &mut Vec<u64>, fn_name: &str, out: &mut String) {
    let bits = value.to_bits();
    let idx = pool.iter().position(|&b| b == bits).unwrap_or_else(|| {
        pool.push(bits);
        pool.len() - 1
    });
    out.push_str(&format!(
        "    movsd xmm{target}, qword ptr [rip + {fn_name}$$const_{idx}]\n",
    ));
}

fn known_constant(name: &str) -> Option<f64> {
    match name {
        "pi" => Some(std::f64::consts::PI),
        "e"  => Some(std::f64::consts::E),
        _    => None,
    }
}

fn validate_expr(expr: &Expr, form: &LetForm) -> Result<(), LetError> {
    match expr {
        Expr::Lit(_) => Ok(()),
        Expr::Var(name) => {
            if known_constant(name).is_some()
                || form.inputs.iter().any(|n| n == name)
                || form.wheres.iter().any(|(n, _)| n == name)
            {
                Ok(())
            } else {
                Err(LetError {
                    message: format!("undefined name '{name}'"),
                    pos: 0,
                })
            }
        }
        Expr::Bin(_, l, r) => { validate_expr(l, form)?; validate_expr(r, form) }
        Expr::Neg(e) => validate_expr(e, form),
        Expr::Call(name, args) => {
            // MVP: no known functions. Any call is an error.
            // (Validate args anyway so the user sees deeper errors if there's a typo inside.)
            for a in args { validate_expr(a, form)?; }
            Err(LetError {
                message: format!("function call '{name}' not yet implemented"),
                pos: 0,
            })
        }
    }
}

fn topo_sort_wheres(wheres: &[(String, Expr)]) -> Result<Vec<usize>, LetError> {
    let n = wheres.len();
    let mut name_to_idx: HashMap<&str, usize> = HashMap::new();
    for (i, (name, _)) in wheres.iter().enumerate() {
        if name_to_idx.insert(name.as_str(), i).is_some() {
            return Err(LetError {
                message: format!("duplicate WHERE binding '{name}'"),
                pos: 0,
            });
        }
    }
    let mut deps: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, (_, expr)) in wheres.iter().enumerate() {
        collect_deps(expr, &name_to_idx, &mut deps[i]);
    }
    // Kahn's. in_deg[i] = how many other bindings i depends on.
    let mut in_deg: Vec<usize> = deps.iter().map(|d| d.len()).collect();
    let mut queue: Vec<usize> = (0..n).filter(|&i| in_deg[i] == 0).collect();
    let mut order = Vec::with_capacity(n);
    while let Some(i) = queue.pop() {
        order.push(i);
        // Anyone depending on i loses one in-degree.
        for j in 0..n {
            if deps[j].contains(&i) {
                in_deg[j] -= 1;
                if in_deg[j] == 0 { queue.push(j); }
            }
        }
    }
    if order.len() != n {
        return Err(LetError {
            message: "circular dependency in WHERE clauses".into(),
            pos: 0,
        });
    }
    Ok(order)
}

fn collect_deps(expr: &Expr, name_to_idx: &HashMap<&str, usize>, deps: &mut Vec<usize>) {
    match expr {
        Expr::Lit(_) => {}
        Expr::Var(n) => {
            if let Some(&idx) = name_to_idx.get(n.as_str()) {
                if !deps.contains(&idx) { deps.push(idx); }
            }
        }
        Expr::Bin(_, l, r) => { collect_deps(l, name_to_idx, deps); collect_deps(r, name_to_idx, deps); }
        Expr::Neg(e) => collect_deps(e, name_to_idx, deps),
        Expr::Call(_, args) => for a in args { collect_deps(a, name_to_idx, deps); },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::parser::parse;

    #[test]
    fn lower_minimal() {
        let form = parse("LET (r) -> (a) = r END").unwrap();
        let asm = lower(&form, "let_test1").unwrap();
        assert!(asm.contains("let_test1:"));
        assert!(asm.contains("movsd xmm0, qword ptr [rcx + 0]"));
        assert!(asm.contains("ret"));
    }

    #[test]
    fn lower_area_of_circle() {
        let form = parse("LET (r) -> (a) = pi * r * r END").unwrap();
        let asm = lower(&form, "let_area").unwrap();
        assert!(asm.contains("mulsd"));
        assert!(asm.contains("let_area$$const_"));
    }

    #[test]
    fn lower_detects_cycle() {
        let e = lower(
            &parse("LET (x) -> (y) = a WHERE a = b WHERE b = a END").unwrap(),
            "let_cycle",
        )
        .unwrap_err();
        assert!(e.message.contains("circular"));
    }

    #[test]
    fn lower_undefined_name() {
        let e = lower(
            &parse("LET (x) -> (y) = z END").unwrap(),
            "let_undef",
        )
        .unwrap_err();
        assert!(e.message.contains("undefined"));
    }

    #[test]
    fn lower_function_call_errors() {
        let e = lower(
            &parse("LET (x) -> (y) = sin(x) END").unwrap(),
            "let_sin",
        )
        .unwrap_err();
        assert!(e.message.contains("sin"));
    }

    #[test]
    fn lower_mbrot_compiles() {
        let form = parse("\
            LET (z_re, z_im, x, y) -> (z_next_re, z_next_im, mag) = \
                re, im, rmag \
                WHERE re   = (z_re * z_re) - (z_im * z_im) + x \
                WHERE im   = (2 * z_re * z_im) + y \
                WHERE rmag = (re * re) + (im * im) \
            END").unwrap();
        let asm = lower(&form, "let_mbrot").unwrap();
        // 4 inputs + 3 wheres = 7 named regs.  Inputs load in reverse
        // memory order (TOS = last-declared input):
        //   xmm0 (z_re) at offset 24 — deepest
        //   xmm1 (z_im) at offset 16
        //   xmm2 (x)    at offset 8
        //   xmm3 (y)    at offset 0 — TOS
        assert!(asm.contains("movsd xmm0, qword ptr [rcx + 24]"));
        assert!(asm.contains("movsd xmm1, qword ptr [rcx + 16]"));
        assert!(asm.contains("movsd xmm2, qword ptr [rcx + 8]"));
        assert!(asm.contains("movsd xmm3, qword ptr [rcx + 0]"));
        assert!(asm.contains("xmm6"));
    }
}
