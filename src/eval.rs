use crate::{Memory, PRECISION, Value, fixed::Fixed};
use std::{
    io::{StdoutLock, Write},
    ops::Neg,
};

macro_rules! error {
    ( $err:expr, $fail:tt ) => {
        eprintln!("Error: {}", $err);
        if $fail {
            return false;
        }
    };
}

macro_rules! math {
    ( $memory:tt, $op:tt, $fail:tt ) => {
        if let Some(rhs) = $memory.stack.pop() && let Some(lhs) = $memory.stack.last_mut() {
            if let (Number(lhs), Number(ref rhs)) = (lhs, rhs) {
                *lhs = &*lhs $op rhs
            } else {
                error!(Error::NaN, $fail);
            }
        } else {
            error!(Error::EmptyStack, $fail);
        }
    };
}

macro_rules! cmp {
    ( $memory:tt, $op:tt, $branch:tt, $out:tt, $fail:tt ) => {
        if let Some(rhs) = $memory.stack.pop() && let Some(lhs) = $memory.stack.pop() {
            if let (Number(lhs), Number(rhs)) = (lhs, rhs) {
                if lhs $op rhs {
                   match &$branch {
                        Number(_) => $memory.stack.push($branch),
                        String(s) => if $fail && !eval(s, $memory, $out, true) {
                            return false;
                        },
                    }
                }
            } else {
                error!(Error::NaN, $fail);
            }
        } else {
            error!(Error::EmptyStack, $fail);
        }
    };
}

pub fn eval(script: &[u8], memory: &mut Memory, out: &mut StdoutLock, fail: bool) -> bool {
    use Value::*;
    let mut i = 0;
    while i < script.len() {
        // dbg!(&memory.stack);
        match &script[i] {
            // basic arithmetics
            b'+' => math!(memory, +, fail),
            b'-' => math!(memory, -, fail),
            b'*' => math!(memory, *, fail),
            b'/' => {
                if let Some(rhs) = memory.stack.pop()
                    && let Some(lhs) = memory.stack.last_mut()
                {
                    if let (Number(lhs), Number(ref rhs)) = (lhs, rhs) {
                        if rhs.is_zero() {
                            error!(Error::ZeroDivision, fail);
                        }
                        *lhs = &*lhs / rhs;
                    } else {
                        error!(Error::NaN, fail);
                    }
                } else {
                    error!(Error::EmptyStack, fail);
                }
            }
            b'%' => {
                if let Some(rhs) = memory.stack.pop()
                    && let Some(lhs) = memory.stack.last_mut()
                {
                    if let (Number(lhs), Number(ref rhs)) = (lhs, rhs) {
                        if rhs.is_zero() {
                            error!(Error::ZeroDivision, fail);
                        }
                        *lhs = &*lhs % rhs;
                    } else {
                        error!(Error::NaN, fail);
                    }
                } else {
                    error!(Error::EmptyStack, fail);
                }
            }
            b'^' => {
                if let Some(rhs) = memory.stack.pop()
                    && let Some(lhs) = memory.stack.last_mut()
                {
                    if let (Number(lhs), Number(rhs)) = (lhs, rhs) {
                        if let Some(val) = lhs.checked_pow(&rhs) {
                            *lhs = val;
                        } else {
                            error!(Error::InvalidExponent, fail);
                        }
                    } else {
                        error!(Error::NaN, fail);
                    }
                } else {
                    error!(Error::EmptyStack, fail);
                }
            }
            // sqrt
            b'v' => {
                if let Some(val) = memory.stack.last_mut() {
                    if let Number(val) = val {
                        if val.is_negative() {
                            error!(Error::NegativeNumber, fail);
                        } else {
                            *val = val.sqrt();
                        }
                    } else {
                        error!(Error::NaN, fail);
                    }
                } else {
                    error!(Error::EmptyStack, fail);
                };
            }
            // duplicate
            b'd' => {
                if let Some(val) = memory.stack.last().cloned() {
                    memory.stack.push(val);
                } else {
                    error!(Error::EmptyStack, fail);
                };
            }
            // clear
            b'c' => memory.stack.clear(),
            // reverse
            b'r' => {
                let n = memory.stack.len();
                if n > 1 {
                    memory.stack.swap(n - 2, n - 1);
                } else {
                    error!(Error::EmptyStack, fail);
                }
            }
            // println
            b'p' => {
                if let Some(val) = memory.stack.pop() {
                    let Ok(_) = writeln!(out, "{}", val) else {
                        std::process::exit(74);
                    };
                } else {
                    error!(Error::EmptyStack, fail);
                }
            }
            // print
            b'n' => {
                if let Some(val) = memory.stack.pop() {
                    let Ok(_) = write!(out, "{}", val) else {
                        std::process::exit(74);
                    };
                } else {
                    error!(Error::EmptyStack, fail);
                }
            }
            // print stack
            b'f' => {
                if !memory.stack.is_empty() {
                    let mut s = memory
                        .stack
                        .iter()
                        .fold(std::string::String::new(), |acc, v| {
                            acc + &v.to_string() + " "
                        });
                    s.pop();
                    if writeln!(out, "{}", s).is_err() {
                        std::process::exit(74);
                    }
                }
            }
            // set precision
            b'k' => {
                if let Some(Number(n)) = memory.stack.pop() {
                    unsafe { PRECISION = n.to_u32_saturating() }
                } else {
                    error!(Error::EmptyStack, fail);
                }
            }
            // save record
            b's' => {
                i += 1;
                if i < script.len() {
                    if let Some(v) = memory.stack.pop() {
                        memory.register[script[i] as usize] = v;
                    } else {
                        error!(Error::EmptyStack, fail);
                    }
                } else {
                    error!(Error::EoF, fail);
                }
            }
            // load record
            b'l' => {
                i += 1;
                if i < script.len() {
                    let v = memory.register[script[i] as usize].clone();
                    memory.stack.push(v);
                } else {
                    error!(Error::EoF, fail);
                }
            }
            // execute
            b'x' => {
                if let Some(v) = memory.stack.pop() {
                    match &v {
                        Number(_) => memory.stack.push(v),
                        String(s) => {
                            if fail && !eval(s, memory, out, true) {
                                return false;
                            }
                        }
                    }
                } else {
                    error!(Error::EmptyStack, fail);
                }
            }
            // comparisons
            b'>' => {
                i += 1;
                if i < script.len() {
                    let branch = memory.register[script[i] as usize].clone();
                    cmp!(memory, <, branch, out, fail);
                } else {
                    error!(Error::EoF, fail);
                }
            }
            b'<' => {
                i += 1;
                if i < script.len() {
                    let branch = memory.register[script[i] as usize].clone();
                    cmp!(memory, >, branch, out, fail);
                } else {
                    error!(Error::EoF, fail);
                }
            }
            b'=' => {
                i += 1;
                if i < script.len() {
                    let branch = memory.register[script[i] as usize].clone();
                    cmp!(memory, ==, branch, out, fail);
                } else {
                    error!(Error::EoF, fail);
                }
            }
            b'!' => {
                if i + 2 < script.len() {
                    let branch = memory.register[script[i + 2] as usize].clone();
                    match script[i + 1] {
                        b'>' => cmp!(memory, >=, branch, out, fail),
                        b'<' => cmp!(memory, <=, branch, out, fail),
                        b'=' => cmp!(memory, !=, branch, out, fail),
                        c => {
                            error!(Error::Unexpected(c), fail);
                        }
                    }
                } else {
                    error!(Error::EoF, fail);
                }
                i += 2;
            }
            // read string
            b'[' => {
                i += 1;
                let mut acc = Vec::new();
                while i < script.len() {
                    let c = match script[i] {
                        b']' => break,
                        b'\\' => {
                            i += 1;
                            match script[i] {
                                b'n' => b'\n',
                                b'r' => b'\r',
                                b't' => b'\t',
                                b'0' => b'\0',
                                c => c,
                            }
                        }
                        c => c,
                    };
                    acc.push(c);
                    i += 1;
                }
                let s = String(acc);
                memory.stack.push(s);
            }
            // comment
            b'#' => {
                while i < script.len() && script[i] != b'\n' && script[i] != b'\r' {
                    i += 1;
                }
                // in case of \n\r or \r\n, the next character would be ignored as whitespace
            }
            // number
            c if *c == b'_' || *c == b'.' || c.is_ascii_digit() => {
                let mut negate = false;
                if *c == b'_' {
                    negate = true;
                    i += 1;
                }
                let start = i;
                while i < script.len() {
                    if !script[i].is_ascii_digit() {
                        break;
                    }
                    i += 1;
                }
                if i < script.len() && script[i] == b'.' {
                    i += 1;
                    while i < script.len() {
                        if !script[i].is_ascii_digit() {
                            break;
                        }
                        i += 1;
                    }
                }
                let mut val = Fixed::from(&script[start..i]);
                if negate {
                    val = val.neg();
                }
                memory.stack.push(Number(val));
                continue;
            }
            c if c.is_ascii_whitespace() => {}
            // invalid
            c => {
                error!(Error::Unexpected(*c), fail);
            }
        }
        i += 1;
    }
    true
}

enum Error {
    EmptyStack,
    NaN,
    ZeroDivision,
    NegativeNumber,
    InvalidExponent,
    EoF,
    Unexpected(u8),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            EmptyStack => write!(f, "empty stack"),
            NaN => write!(f, "not a number"),
            ZeroDivision => write!(f, "division by zero"),
            NegativeNumber => write!(f, "negative number"),
            InvalidExponent => write!(f, "invalid exponent"),
            EoF => write!(f, "unexpected end of input"),
            Unexpected(c) => write!(f, "unexpected character: {}", *c as char),
        }
    }
}
