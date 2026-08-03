mod eval;
mod fixed;

pub use eval::eval;

/// Exit code for the I/O issues
pub const IO_EXIT: i32 = 2;

/// The precision (number of digits after the decimal point).
/// Reading and writing it is always safe because there is no concurrency.
static mut PRECISION: u32 = 0;
static mut OUTPUT_RADIX: u32 = 10;
static mut INPUT_RADIX: u32 = 10;

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
    Number(fixed::Fixed),
    String(Vec<u8>),
}

impl Default for Value {
    fn default() -> Self {
        Value::Number(Default::default())
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Value::*;
        match self {
            Number(n) => write!(f, "{}", n),
            String(s) => {
                let s = std::string::String::from_utf8_lossy(s);
                write!(f, "{}", s)
            }
        }
    }
}
