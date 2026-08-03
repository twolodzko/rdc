use crate::{INPUT_RADIX, IO_EXIT, Memory, OUTPUT_RADIX, PRECISION, Value, fixed::Fixed};
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

impl Memory {
    /// Pop two numbers from the stack or fail
    fn two_numbers(&mut self) -> Result<(Fixed, Fixed), Error> {
        if let Some(rhs) = self.stack.pop()
            && let Some(lhs) = self.stack.pop()
        {
            use Value::Number;
            if let (Number(lhs), Number(rhs)) = (lhs, rhs) {
                Ok((lhs, rhs))
            } else {
                Err(Error::NaN)
            }
        } else {
            Err(Error::EmptyStack)
        }
    }
}

pub fn eval(script: &[u8], memory: &mut Memory, out: &mut StdoutLock, fail: bool) -> bool {
    use Value::*;
    let mut queue = vec![(script.to_vec(), 0)];
    while let Some((mut cmds, mut i)) = queue.pop() {
        while i < cmds.len() {
            /// execute the macro at the given register address
            macro_rules! exec {
                ( $addr:tt ) => {
                    match $addr {
                        Number(_) => memory.stack.push($addr),
                        String(s) => {
                            i += 1;
                            if i < cmds.len() {
                                // only push to queue if there's anything more to run
                                queue.push((cmds, i));
                            }
                            i = 0;
                            cmds = s;
                            continue;
                        }
                    }
                };
            }

            match &cmds[i] {
                // basic arithmetic
                b'+' => {
                    match memory.two_numbers() {
                        Ok((ref lhs, ref rhs)) => memory.stack.push(Number(lhs + rhs)),
                        Err(err) => {
                            error!(err, fail);
                        }
                    };
                }
                b'-' => {
                    match memory.two_numbers() {
                        Ok((ref lhs, ref rhs)) => memory.stack.push(Number(lhs - rhs)),
                        Err(err) => {
                            error!(err, fail);
                        }
                    };
                }
                b'*' => {
                    match memory.two_numbers() {
                        Ok((ref lhs, ref rhs)) => memory.stack.push(Number(lhs * rhs)),
                        Err(err) => {
                            error!(err, fail);
                        }
                    };
                }
                b'/' => {
                    match memory.two_numbers() {
                        Ok((ref lhs, ref rhs)) => {
                            if rhs.is_zero() {
                                error!(Error::ZeroDivision, fail);
                            } else {
                                memory.stack.push(Number(lhs / rhs))
                            }
                        }
                        Err(err) => {
                            error!(err, fail);
                        }
                    };
                }
                b'%' => {
                    match memory.two_numbers() {
                        Ok((ref lhs, ref rhs)) => {
                            if rhs.is_zero() {
                                error!(Error::ZeroDivision, fail);
                            } else {
                                memory.stack.push(Number(lhs % rhs))
                            }
                        }
                        Err(err) => {
                            error!(err, fail);
                        }
                    };
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
                            std::process::exit(IO_EXIT);
                        };
                    } else {
                        error!(Error::EmptyStack, fail);
                    }
                }
                // print
                b'n' => {
                    if let Some(val) = memory.stack.pop() {
                        let Ok(_) = write!(out, "{}", val) else {
                            std::process::exit(IO_EXIT);
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
                            std::process::exit(IO_EXIT);
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
                // set input radix
                b'i' => {
                    if let Some(Number(n)) = memory.stack.pop() {
                        unsafe { INPUT_RADIX = n.to_u32_saturating().clamp(2, 16) }
                    } else {
                        error!(Error::EmptyStack, fail);
                    }
                }
                // set output radix
                b'o' => {
                    if let Some(Number(n)) = memory.stack.pop() {
                        unsafe { OUTPUT_RADIX = n.to_u32_saturating().clamp(2, 16) }
                    } else {
                        error!(Error::EmptyStack, fail);
                    }
                }
                // save record
                b's' => {
                    i += 1;
                    if i < cmds.len() {
                        if let Some(v) = memory.stack.pop() {
                            memory.register[cmds[i] as usize] = v;
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
                    if i < cmds.len() {
                        let v = memory.register[cmds[i] as usize].clone();
                        memory.stack.push(v);
                    } else {
                        error!(Error::EoF, fail);
                    }
                }
                // execute
                b'x' => {
                    if let Some(v) = memory.stack.pop() {
                        exec!(v);
                    } else {
                        error!(Error::EmptyStack, fail);
                    }
                }
                // comparisons
                b'>' => {
                    i += 1;
                    if i < cmds.len() {
                        let branch = memory.register[cmds[i] as usize].clone();
                        match memory.two_numbers() {
                            Ok((ref lhs, ref rhs)) => {
                                if lhs < rhs {
                                    exec!(branch)
                                }
                            }
                            Err(err) => {
                                error!(err, fail);
                            }
                        };
                    } else {
                        error!(Error::EoF, fail);
                    }
                }
                b'<' => {
                    i += 1;
                    if i < cmds.len() {
                        let branch = memory.register[cmds[i] as usize].clone();
                        match memory.two_numbers() {
                            Ok((ref lhs, ref rhs)) => {
                                if lhs > rhs {
                                    exec!(branch)
                                }
                            }
                            Err(err) => {
                                error!(err, fail);
                            }
                        };
                    } else {
                        error!(Error::EoF, fail);
                    }
                }
                b'=' => {
                    i += 1;
                    if i < cmds.len() {
                        let branch = memory.register[cmds[i] as usize].clone();
                        match memory.two_numbers() {
                            Ok((ref lhs, ref rhs)) => {
                                if lhs == rhs {
                                    exec!(branch)
                                }
                            }
                            Err(err) => {
                                error!(err, fail);
                            }
                        };
                    } else {
                        error!(Error::EoF, fail);
                    }
                }
                b'!' => {
                    if i + 2 < cmds.len() {
                        let branch = memory.register[cmds[i + 2] as usize].clone();
                        match cmds[i + 1] {
                            b'>' => {
                                i += 2;
                                match memory.two_numbers() {
                                    Ok((ref lhs, ref rhs)) => {
                                        if lhs >= rhs {
                                            exec!(branch)
                                        }
                                    }
                                    Err(err) => {
                                        error!(err, fail);
                                    }
                                };
                            }
                            b'<' => {
                                i += 2;
                                match memory.two_numbers() {
                                    Ok((ref lhs, ref rhs)) => {
                                        if lhs <= rhs {
                                            exec!(branch)
                                        }
                                    }
                                    Err(err) => {
                                        error!(err, fail);
                                    }
                                };
                            }
                            b'=' => {
                                i += 2;
                                match memory.two_numbers() {
                                    Ok((ref lhs, ref rhs)) => {
                                        if lhs != rhs {
                                            exec!(branch)
                                        }
                                    }
                                    Err(err) => {
                                        error!(err, fail);
                                    }
                                };
                            }
                            c => {
                                error!(Error::Unexpected(c), fail);
                            }
                        }
                    } else {
                        error!(Error::EoF, fail);
                    }
                }
                // read string
                b'[' => {
                    i += 1;
                    let mut brackets = 1;
                    let mut acc = Vec::new();
                    while i < cmds.len() {
                        let mut c = cmds[i];
                        match c {
                            b'[' => {
                                brackets += 1;
                            }
                            b']' => {
                                brackets -= 1;
                                if brackets == 0 {
                                    break;
                                }
                            }
                            b'\\' => {
                                i += 1;
                                c = match cmds[i] {
                                    b'n' => b'\n',
                                    b'r' => b'\r',
                                    b't' => b'\t',
                                    b'0' => b'\0',
                                    c => c,
                                }
                            }
                            _ => {}
                        };
                        acc.push(c);
                        i += 1;
                    }
                    let s = String(acc);
                    memory.stack.push(s);
                }
                // comment
                b'#' => {
                    while i < cmds.len() && cmds[i] != b'\n' && cmds[i] != b'\r' {
                        i += 1;
                    }
                    // in case of \n\r or \r\n, the next character would be ignored as whitespace
                }
                // quit
                b'q' => return true,
                // number
                c if *c == b'_' || *c == b'.' || is_hex_digit(*c) => {
                    let mut negate = false;
                    if *c == b'_' {
                        negate = true;
                        i += 1;
                    }
                    let start = i;
                    while i < cmds.len() {
                        if !is_hex_digit(cmds[i]) {
                            break;
                        }
                        i += 1;
                    }
                    if i < cmds.len() && cmds[i] == b'.' {
                        i += 1;
                        while i < cmds.len() {
                            if !is_hex_digit(cmds[i]) {
                                break;
                            }
                            i += 1;
                        }
                    }
                    let radix = unsafe { INPUT_RADIX };
                    let mut val = Fixed::parse(&cmds[start..i], radix);
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

/// Check if the string byte is a hexadecimal digit
fn is_hex_digit(c: u8) -> bool {
    c.is_ascii_digit() || matches!(c, b'A'..=b'F')
}
