//! # Eva
//!
//! Evaluate expression.
//!
//! ## Examples
//!
//! ```
//! use eva::lex::FunctionContext;
//!
//! let ctx = FunctionContext::default();
//! assert_eq!(eva::eval_expr(&ctx, 10, "1 + 1", None), Ok(2.));
//! ```
/*
 *  eva - an easy to use calculator REPL similar to bc(1)
 *  Copyright (C) 2019  Akshay Oppiliappan <nerdypepper@tuta.io>
 */
use std::cmp::Ordering;

pub mod error;
pub mod lex;
mod parse;

pub use crate::error::CalcError;
use crate::lex::*;
use crate::parse::*;

fn autobalance_parens(input: &str) -> Result<String, CalcError> {
    let mut balanced = String::from(input);
    let mut left_parens = 0;
    let mut right_parens = 0;
    for letter in input.chars() {
        if letter == '(' {
            left_parens += 1;
        } else if letter == ')' {
            right_parens += 1;
        }
    }

    match left_parens.cmp(&right_parens) {
        Ordering::Greater => {
            let extras = ")".repeat(left_parens - right_parens);
            balanced.push_str(&extras[..]);
            Ok(balanced)
        }
        Ordering::Equal => Ok(balanced),
        Ordering::Less => Err(CalcError::Syntax("Mismatched parentheses!".into())),
    }
}

/// Evaluate math expression. Main entry function for eva.
pub fn eval_expr(
    ctx: &FunctionContext,
    fix: usize,
    input: &str,
    prev_ans: Option<f64>,
) -> Result<f64, CalcError> {
    let input = input.trim().replace(' ', "");
    if input == "help" {
        return Err(CalcError::Help);
    }
    if input.is_empty() {
        return Ok(0.);
    }
    let input = autobalance_parens(&input[..])?;
    let lexed = lexer(&input[..], prev_ans)?;
    let postfixed = to_postfix(lexed)?;
    let evaled = eval_postfix(ctx, postfixed)?;
    let evaled_fixed = format!("{:.*}", fix, evaled).parse::<f64>().unwrap();
    Ok(evaled_fixed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MathError;

    pub fn eval(input: &str, prev_ans: Option<f64>) -> Result<f64, CalcError> {
        let ctx = FunctionContext::default();
        let fix = 10;
        let ans = eval_expr(&ctx, fix, input, prev_ans)?;
        Ok(format!("{:.*}", fix, ans).parse().unwrap())
    }

    #[test]
    fn basic_ops() {
        let evaled = eval("6*2 + 3 + 12 -3", Some(0f64));
        assert_eq!(evaled, Ok(24.));
    }
    #[test]
    fn trignometric_fns() {
        let evaled = eval("sin(30) + tan(45", Some(0f64));
        assert_eq!(evaled, Ok(1.5));
    }
    #[test]
    fn brackets() {
        let evaled = eval("(((1 + 2 + 3) ^ 2 ) - 4)", Some(0f64));
        assert_eq!(evaled, Ok(32.));
    }
    #[test]
    fn exponentiation() {
        let evaled = eval("2 ** 2 ** 3", None);
        assert_eq!(evaled, Ok(256.)); // 2^(2^3), not (2^2)^3
    }
    #[test]
    fn floating_ops() {
        let evaled = eval("1.2816 + 1 + 1.2816/1.2", Some(0f64));
        assert_eq!(evaled, Ok(3.3496));
    }
    #[test]
    fn inverse_trignometric_fns() {
        let evaled = eval("deg(asin(1) + acos(1))", Some(0f64));
        assert_eq!(evaled, Ok(90.));
    }
    #[test]
    fn sigmoid_fns() {
        let evaled = eval("1 / (1 + e^-7)", Some(0f64));
        assert_eq!(evaled, Ok(0.9990889488));
    }
    #[test]
    fn prev_ans() {
        let evaled = eval("_ + 9", Some(9f64));
        assert_eq!(evaled, Ok(18.0));
    }
    #[test]
    fn eval_with_zero_prev() {
        let evaled = eval("9 + _ ", Some(0f64));
        assert_eq!(evaled, Ok(9.));
    }
    #[test]
    fn eval_const_multiplication() {
        let evaled = eval("e2", None);
        assert_eq!(evaled, Ok(5.4365636569));
    }
    #[test]
    fn eval_round() {
        let evaled = eval("round(0.5)+round(2.4)", None);
        assert_eq!(evaled, Ok(3.));
    }
    #[test]
    fn eval_exp2() {
        let evaled = eval("exp2(8)", None);
        assert_eq!(evaled, Ok(256.));
    }
    #[test]
    fn eval_exp() {
        let evaled = eval("exp(3)", None);
        assert_eq!(evaled, Ok(20.0855369232));
    }
    #[test]
    fn eval_e_times_n() {
        let evaled = eval("e0", None);
        assert_eq!(evaled, Ok(0.));
    }
    #[test]
    fn eval_factorial_large() {
        let evaled = eval("21!", None);
        assert_eq!(evaled, Ok(51_090_942_171_709_440_000.0));
    }
    #[test]
    fn eval_nroot() {
        let evaled = eval("nroot(27, 3)", None);
        assert_eq!(evaled, Ok(3.));
    }
    #[test]
    fn eval_log_n_base() {
        let evaled = eval("log(2^16,4)", None);
        assert_eq!(evaled, Ok(8.));
    }
    #[test]
    fn eval_log_n_brackets() {
        let evaled = eval("log(1+(2^16),4)", None);
        assert_eq!(evaled, Ok(8.0000110068));
    }
    #[test]
    fn eval_mismatched_parens_in_multiarg_fn() {
        let evaled = eval("log(1+(2^16, 4)", None);
        assert_eq!(
            evaled,
            Err(CalcError::Syntax("Mismatched parentheses!".to_string()))
        );
    }
    #[test]
    fn eval_comma_without_multiarg_fn() {
        let evaled = eval("1+(2^16, 4)", None);
        assert_eq!(
            evaled,
            Err(CalcError::Syntax("Mismatched parentheses!".to_string()))
        );
    }
    #[test]
    fn eval_unexpected_comma() {
        let evaled = eval("(1+1,2+2)", None);
        assert_eq!(
            evaled,
            Err(CalcError::Parser(
                "Too many operators, too few operands".to_string()
            ))
        );
    }
    #[test]
    fn eval_nroot_expr_on_both_sides() {
        let evaled = eval("nroot(2+2,4+e^2)", None);
        assert_eq!(evaled, Ok(1.1294396449));
    }
    #[test]
    fn eval_comma_left_paren_mixup() {
        let evaled = eval("exp 2,3)", None);
        assert_eq!(
            evaled,
            Err(CalcError::Syntax("Mismatched parentheses!".to_string()))
        );
        let evaled = eval("exp,2,3)", None);
        assert_eq!(
            evaled,
            Err(CalcError::Syntax("Mismatched parentheses!".to_string()))
        );
    }
    #[test]
    fn eval_log2() {
        let evaled = eval("log2(1024)", None);
        assert_eq!(evaled, Ok(10.));
    }
    #[test]
    fn eval_log10() {
        let evaled = eval("log10(1000)", None);
        assert_eq!(evaled, Ok(3.));
    }
    #[test]
    fn eval_empty_argument() {
        let evaled = eval("log(2,,3)", None);
        assert_eq!(evaled, Err(CalcError::Syntax("Empty argument".to_string())));
    }
    #[test]
    fn eval_mismatched_args() {
        let evaled = eval("nroot(23,3,4)", None);
        assert_eq!(
            evaled,
            Err(CalcError::Parser(
                "Too many operators, too few operands".to_string()
            ))
        );
        let evaled = eval("nroot(23)", None);
        assert_eq!(
            evaled,
            Err(CalcError::Parser(
                "To few arguments for function, need 2".to_string()
            ))
        );
    }
    #[test]
    fn eval_negative_factorial() {
        let evaled = eval("-1!", None);
        assert_eq!(Err(CalcError::Math(MathError::OutOfBounds)), evaled);
    }
}
