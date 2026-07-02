use rug::ops::{NegAssign, PowAssign};
use std::io::{StdoutLock, Write};

static mut SCALE: usize = 10;
static mut PRECISION: u32 = 53; // f64;

#[derive(Debug)]
pub struct Memory {
    stack: Vec<Value>,
    register: [Value; 256],
}

impl Default for Memory {
    fn default() -> Memory {
        Memory {
            stack: Vec::new(),
            register: std::array::from_fn(|_| Default::default()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Number(rug::Float),
    String(Vec<u8>),
}

impl Default for Value {
    fn default() -> Self {
        Value::Number(rug::Float::new(unsafe { PRECISION }))
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Value::*;
        match self {
            Number(n) => {
                let scale = unsafe { SCALE };
                let (negative, string, exponent) = n.to_sign_string_exp(10, None);
                let mut tmp: std::string::String;
                let r = match exponent {
                    // exponent is larger than the scale
                    Some(e) if e.unsigned_abs() as usize >= scale => {
                        &n.to_string_radix(10, Some(scale))
                    }
                    // decimal point is before the representation, it needs a prefix
                    Some(e) if e.is_negative() => {
                        tmp = "0.".into();
                        for _ in 0..(e.unsigned_abs() as usize) {
                            tmp.push('0');
                        }
                        tmp.push_str(&string);
                        tmp.truncate(scale + 2);
                        tmp.trim_end_matches("0").trim_end_matches('.')
                    }
                    // decimal point is inside the representation, need to split
                    Some(e) if string.len() > e as usize => {
                        let (int, frac): (&str, &str) = if e == 0 {
                            ("0", &string)
                        } else {
                            let (int, mut frac) = string.split_at(e as usize);
                            if frac.len() > scale {
                                (frac, _) = frac.split_at(scale);
                            }
                            (int, frac)
                        };
                        let frac = frac.trim_end_matches("0");
                        if frac.is_empty() {
                            tmp = int.to_string();
                        } else {
                            tmp = format!("{}.{}", int, frac);
                        }
                        tmp.as_str()
                    }
                    // decimal point is after the representation, it misses some zeros
                    Some(e) => {
                        tmp = string.to_string();
                        for _ in 0..(e as usize).saturating_sub(string.len()) {
                            tmp.push('0');
                        }
                        tmp.as_str()
                    }
                    // there is no decimal point
                    _ => string.as_str(),
                };
                if negative {
                    write!(f, "-{}", r)
                } else {
                    write!(f, "{}", r)
                }
            }
            String(s) => {
                let s = unsafe { std::str::from_utf8_unchecked(s) };
                write!(f, "{}", s)
            }
        }
    }
}

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
            if let (Number(lhs), Number(rhs)) = (lhs, rhs) {
                *lhs $op rhs
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
            b'+' => math!(memory, +=, fail),
            b'-' => math!(memory, -=, fail),
            b'*' => math!(memory, *=, fail),
            b'/' => math!(memory, /=, fail),
            b'%' => math!(memory, %=, fail),
            b'^' => {
                if let Some(rhs) = memory.stack.pop()
                    && let Some(lhs) = memory.stack.last_mut()
                {
                    if let (Number(lhs), Number(rhs)) = (lhs, rhs) {
                        lhs.pow_assign(rhs);
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
                        val.sqrt_mut();
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
                    if let Some(u) = n.to_u32_saturating() {
                        unsafe { PRECISION = u }
                    } else {
                        error!(Error::NaN, fail);
                    }
                } else {
                    error!(Error::EmptyStack, fail);
                }
            }
            // set scale
            b'j' => {
                if let Some(Number(n)) = memory.stack.pop() {
                    if let Some(u) = n.to_u32_saturating() {
                        unsafe { SCALE = u as usize }
                    } else {
                        error!(Error::NaN, fail);
                    }
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
            c if *c == b'_' || c.is_ascii_digit() => {
                let mut negate = false;
                if *c == b'_' {
                    negate = true;
                    i += 1;
                }
                let start = i;
                while i < script.len() {
                    match script[i] {
                        c if c.is_ascii_digit() => {}
                        _ => break,
                    }
                    i += 1;
                }
                if i < script.iter().len() && script[i] == b'.' {
                    i += 1;
                    while i < script.len() {
                        match script[i] {
                            c if c.is_ascii_digit() => {}
                            _ => break,
                        }
                        i += 1;
                    }
                }
                if let Ok(val) = rug::Float::parse(&script[start..i]) {
                    let mut f = rug::Float::with_val(unsafe { PRECISION }, val);
                    if negate {
                        f.neg_assign();
                    }
                    memory.stack.push(Number(f));
                } else {
                    error!(Error::NaN, fail);
                };
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
    EoF,
    Unexpected(u8),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            EmptyStack => write!(f, "empty stack"),
            NaN => write!(f, "not a number"),
            EoF => write!(f, "unexpected end of input"),
            Unexpected(c) => write!(f, "unexpected character: {}", *c as char),
        }
    }
}
