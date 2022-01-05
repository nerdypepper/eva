/*
 *  eva - an easy to use calculator REPL similar to bc(1)
 *  Copyright (C) 2019  Akshay Oppiliappan <nerdypepper@tuta.io>
 */

/* imports */
// std
use std::f64;
use std::fs::create_dir_all;
use std::path::PathBuf;

// modules
mod error;
mod format;
mod lex;
mod parse;
mod readline;
use crate::error::{handler, CalcError};
use crate::format::*;
use crate::lex::*;
use crate::parse::*;
use crate::readline::*;

// extern crates
use clap::{App, AppSettings, Arg};
use directories::{ProjectDirs, UserDirs};
use lazy_static::lazy_static;
use rustyline::error::ReadlineError;

/* end of imports */

struct Configuration {
    radian_mode: bool,
    fix: usize,
    base: usize,
    input: String,
}

#[cfg(not(test))]
lazy_static! {
    static ref CONFIGURATION: Configuration = parse_arguments();
}

#[cfg(test)]
lazy_static! {
    static ref CONFIGURATION: Configuration = Configuration {
        radian_mode: false,
        fix: 10,
        base: 10,
        input: "".to_string(),
    };
}

fn main() {
    if !CONFIGURATION.input.is_empty() {
        // command mode //
        let evaled = eval_math_expression(&CONFIGURATION.input[..], Some(0.));
        match evaled {
            Ok(ans) => pprint(ans),
            Err(e) => {
                eprintln!("{}", handler(e));
                std::process::exit(1);
            }
        };
    } else {
        // REPL mode //
        // create fancy readline
        let mut rl = create_readline();

        // previous answer
        let mut prev_ans = None;

        // handle history storage
        let eva_dirs = ProjectDirs::from("com", "NerdyPepper", "eva").unwrap();
        let eva_data_dir = eva_dirs.data_dir();
        let eva_cache_dir = eva_dirs.cache_dir();
        let mut history_path = PathBuf::from(eva_data_dir);
        let mut previous_ans_path = PathBuf::from(eva_cache_dir);

        if let Err(_) = create_dir_all(eva_data_dir) {
            history_path = PathBuf::from(UserDirs::new().unwrap().home_dir());
        }
        if let Err(_) = create_dir_all(eva_cache_dir) {
            previous_ans_path = PathBuf::from(UserDirs::new().unwrap().home_dir());
        }
        history_path.push("history.txt");
        previous_ans_path.push("previous_ans.txt");

        if let Err(err) = std::fs::write(&previous_ans_path, "0") {
            println!("Could not write to previous_ans_path");
            println!("{:?}", err);
            std::process::exit(1);
        }

        if rl.load_history(history_path.as_path()).is_err() {
            println!("No previous history.")
        };

        // repl loop begins here
        loop {
            let readline = rl.readline("> ");
            match readline {
                Ok(line) => {
                    rl.add_history_entry(line.as_str());
                    let evaled = eval_math_expression(&line[..], prev_ans);
                    match evaled {
                        Ok(ans) => {
                            use std::fs::OpenOptions;
                            use std::io::Write;
                            prev_ans = Some(ans);
                            pprint(ans);
                            match OpenOptions::new()
                                .write(true)
                                .create(true)
                                .open(&previous_ans_path)
                            {
                                Ok(mut file) => {
                                    if let Err(err) = writeln!(file, "{}", ans) {
                                        println!(
                                            "Error while writing previous answer to file: {}",
                                            err
                                        )
                                    }
                                }
                                Err(err) => {
                                    println!("Error while writing previous answer to file: {}", err)
                                }
                            }
                        }
                        Err(e) => println!("{}", handler(e)),
                    };
                }
                Err(ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    break;
                }
                Err(ReadlineError::Eof) => break,
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }
        rl.save_history(history_path.as_path()).unwrap();
    }
}

fn parse_arguments() -> Configuration {
    let config = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .global_setting(AppSettings::ColoredHelp)
        .arg(
            Arg::with_name("fix")
                .short("f")
                .long("fix")
                .takes_value(true)
                .value_name("FIX")
                .help("set number of decimal places in the output"),
        )
        .arg(
            Arg::with_name("base")
                .short("b")
                .long("base")
                .takes_value(true)
                .value_name("RADIX")
                .help("set the radix of calculation output (1 - 36)"),
        )
        .arg(
            Arg::with_name("INPUT")
                .help("optional expression string to run eva in command mode")
                .index(1),
        )
        .arg(
            Arg::with_name("radian")
                .short("r")
                .long("radian")
                .help("set eva to radian mode"),
        )
        .get_matches();

    let mut input = String::new();
    if let Some(i) = config.value_of("INPUT") {
        input.push_str(i);
    };
    Configuration {
        radian_mode: config.is_present("radian"),
        fix: config.value_of("fix").unwrap_or("10").parse().unwrap(),
        base: config.value_of("base").unwrap_or("10").parse().unwrap(),
        input,
    }
}

pub fn eval_math_expression(input: &str, prev_ans: Option<f64>) -> Result<f64, CalcError> {
    let input = input.trim().replace(" ", "");
    if input == "help" {
        return Err(CalcError::Help);
    }
    if input.is_empty() {
        return Ok(0.);
    }
    let input = format::autobalance_parens(&input[..])?;
    let lexed = lexer(&input[..], prev_ans)?;
    let postfixed = to_postfix(lexed)?;
    let evaled = eval_postfix(postfixed)?;
    let evaled_fixed = format!("{:.*}", CONFIGURATION.fix, evaled)
        .parse::<f64>()
        .unwrap();
    Ok(evaled_fixed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_ops() {
        let evaled = eval_math_expression("6*2 + 3 + 12 -3", Some(0f64)).unwrap();
        assert_eq!(24., evaled);
    }
    #[test]
    fn trignometric_fns() {
        let evaled = eval_math_expression("sin(30) + tan(45", Some(0f64)).unwrap();
        assert_eq!(1.5, evaled);
    }
    #[test]
    fn brackets() {
        let evaled = eval_math_expression("(((1 + 2 + 3) ^ 2 ) - 4)", Some(0f64)).unwrap();
        assert_eq!(32., evaled);
    }
    #[test]
    fn exponentiation() {
        let evaled = eval_math_expression("2 ** 2 ** 3", None).unwrap();
        assert_eq!(256., evaled); // 2^(2^3), not (2^2)^3
    }
    #[test]
    fn floating_ops() {
        let evaled = eval_math_expression("1.2816 + 1 + 1.2816/1.2", Some(0f64)).unwrap();
        assert_eq!(3.3496, evaled);
    }
    #[test]
    fn inverse_trignometric_fns() {
        let evaled = eval_math_expression("deg(asin(1) + acos(1))", Some(0f64)).unwrap();
        assert_eq!(90., evaled);
    }
    #[test]
    fn sigmoid_fns() {
        let evaled = eval_math_expression("1 / (1 + e^-7)", Some(0f64)).unwrap();
        assert_eq!(0.9990889488, evaled);
    }
    #[test]
    fn prev_ans() {
        let evaled = eval_math_expression("_ + 9", Some(9f64)).unwrap();
        assert_eq!(18.0, evaled);
    }
    #[test]
    fn eval_with_zero_prev() {
        let evaled = eval_math_expression("9 + _ ", Some(0f64)).unwrap();
        assert_eq!(9., evaled);
    }
    #[test]
    fn eval_const_multiplication() {
        let evaled = eval_math_expression("e2", None).unwrap();
        assert_eq!(5.4365636569, evaled);
    }
    #[test]
    fn eval_round() {
        let evaled = eval_math_expression("round(0.5)+round(2.4)", None).unwrap();
        assert_eq!(3., evaled);
    }
    #[test]
    fn eval_exp2() {
        assert_eq!(
            256.,
            eval_math_expression("exp2(8)", None).unwrap()
        );
    }
    #[test]
    fn eval_exp() {
        assert_eq!(
            20.0855369232 as f64,
            eval_math_expression("exp(3)", None).unwrap()
        );
    }
    #[test]
    fn eval_e_times_n() {
        assert_eq!(
            0. as f64,
            eval_math_expression("e0", None).unwrap()
        );
    }
    #[test]
    fn eval_two_arg_fn() {
        let evaled = eval_math_expression("nroot(27, 3)", None).unwrap();
        assert_eq!(3., evaled);
    }
    #[test]
    fn eval_log_n_base() {
        let evaled = eval_math_expression("log(2^16, 4)", None).unwrap();
        assert_eq!(8., evaled);
    }
    #[test]
    fn eval_log_n_brackets() {
        assert_eq!(
            8.0000110068 as f64,
            eval_math_expression("log(1+(2^16),4)", None).unwrap()
        );
    }
    #[test]
    fn eval_mismatched_parens_in_multiarg_fn() {
        assert!(
            match eval_math_expression("log(1+(2^16, 4)", None) {
                Err(CalcError::Syntax(_)) => true,
                _ => false,
            }
        );
    }
    #[test]
    fn eval_comma_without_multiarg_fn() {
        assert!(
            match eval_math_expression("1+(2^16, 4)", None) {
                Err(CalcError::Syntax(_)) => true,
                _ => false,
            }
        )
    }
    #[test]
    fn eval_unexpected_comma() {
        assert!(
            match eval_math_expression("(1+1,2+2)", None) {
                Err(CalcError::Parser(y)) => {
                    assert_eq!("Too many operators, Too little operands", y);
                    true
                },
                _ => false,
            }
        )
    }
    #[test]
    fn eval_nroot_expr_on_both_sides() {
        assert_eq!(
            1.1294396449 as f64,
            eval_math_expression("nroot(2+2,4+e^2)", None).unwrap()
        );
    }
    #[test]
    fn eval_comma_left_paren_mixup() {
        assert!(
            match eval_math_expression("exp 2,3)", None) {
                Err(CalcError::Syntax(y)) => {
                    assert_eq!("Mismatched parentheses!", y);
                    true
                },
                _ => false,
            }
        );
        assert!(
            match eval_math_expression("exp,2,3)", None) {
                Err(CalcError::Syntax(y)) => {
                    assert_eq!("Mismatched parentheses!", y);
                    true
                },
                _ => false,
            }
        );
    }
    #[test]
    fn eval_log10() {
        assert_eq!(
            3 as f64,
            eval_math_expression("log10(1000)", None).unwrap()
        );
    }
    #[test]
    fn eval_mismatched_args() {
        assert!(
            match eval_math_expression("nroot(23,3,4)", None) {
                Err(CalcError::Parser(y)) => {
                    assert_eq!("Too many operators, Too little operands", y);
                    true
                },
                _ => false,
            }
        );
        assert!(
            match eval_math_expression("nroot(23)", None) {
                Err(CalcError::Parser(y)) => {
                    assert_eq!("Too few arguments (1) for function nroot (requires 2)!", y);
                    true
                },
                _ => false,
            }
        );
    }
}
