use crate::{OUTPUT_RADIX, PRECISION};
use num_bigint::{BigInt, Sign};
use std::{
    borrow::Cow,
    ops::{Add, Div, Mul, Neg, Rem, Sub},
};

const TEN: BigInt = BigInt::new_const(10);

/// Represents the floating-point number x as a ratio x = value / 10^precision
#[derive(Debug, Clone)]
pub struct Fixed {
    value: BigInt,
    precision: u32,
}

impl Fixed {
    pub fn new(value: BigInt, mut precision: u32) -> Fixed {
        if value == BigInt::ZERO {
            precision = 0
        }
        Fixed { value, precision }
    }

    /// Parse vector of bytes assuming that it contains
    /// only of digits possibly separated by a single dot.
    pub fn parse(bytes: &[u8], radix: u32) -> Fixed {
        let mut value = BigInt::ZERO;
        let mut precision = 0;
        let mut position = BigInt::ONE;
        let base = BigInt::from(radix);
        let mut i = bytes.len();
        while i != 0 {
            i -= 1;
            match bytes[i] {
                b'.' => {
                    precision = (bytes.len() - i - 1) as u32;
                    break;
                }
                b'0' => {}
                v => value += &position * to_digit(v),
            }
            position *= &base;
        }
        while i != 0 {
            i -= 1;
            value += &position * to_digit(bytes[i]);
            position *= &base;
        }
        if precision > 0 && radix != 10 {
            value = value * TEN.pow(precision) / base.pow(precision);
        }
        Fixed::new(value, precision)
    }

    fn to_string_radix(&self, radix: u32) -> String {
        let (sign, value) = if self.is_negative() {
            ("-", self.value.clone().neg())
        } else {
            ("", self.value.clone())
        };
        if self.precision == 0 {
            format!("{}{}", sign, int_to_string(value, radix))
        } else {
            let digits = count_digits(value.clone());
            if digits < self.precision {
                let s = frac_to_string(value, radix, self.precision);
                format!("{}.{}", sign, s)
            } else if radix == 10 {
                let s = value.to_string();
                let (mut int, frac) = s.split_at(s.len() - self.precision as usize);
                if int == "0" {
                    int = "";
                }
                format!("{}{}.{}", sign, int, frac)
            } else {
                let base = TEN.pow(self.precision);
                let mut int = int_to_string(&value / &base, radix);
                let frac = frac_to_string(&value % &base, radix, self.precision);
                if int == "0" {
                    int.clear();
                }
                format!("{}{}.{}", sign, int, frac)
            }
        }
    }

    pub fn sqrt(&self) -> Fixed {
        if self.value == BigInt::ZERO {
            return self.clone();
        }

        // the decimal number is represented by a fractional one x/n
        // where the scaling factor is n=10^p and p is the precision (number of decimal points)
        // and the number stored as an integer y=(x/n)*n
        // by the property of square root: sqrt(ab) = sqrt(a) * sqrt(b)
        // so if we want to keep the precision to be p, we need to take
        // sqrt(y * n) = sqrt(x/n * n * n) = sqrt(n/x) * sqrt(n^2) = sqrt(x/n) * n
        // so we need to multiply and divide by n for the scaling to remain unchanged

        let prec = self.precision.max(unsafe { PRECISION });
        let scaling = TEN.pow(2 * prec);
        let value = (&self.value * self.scaling() * scaling).sqrt() / self.scaling();
        Fixed::new(value, prec)
    }

    /// Raise the value to the rhs power. Ignore the fractional part of the exponent.
    /// The scale of the result is equal to scale.
    pub fn checked_pow(&self, rhs: &Fixed) -> Option<Fixed> {
        // keep only the integer part of exponent
        if let Ok(exp) = u32::try_from(rhs.value_truncated()) {
            match exp {
                0 => Some(Fixed {
                    value: BigInt::ONE,
                    precision: 0,
                }),
                1 => Some(self.clone()),
                _ => {
                    let prec = unsafe { PRECISION }.max(self.precision);
                    let mut pow = Fixed::new(self.value.pow(exp), self.precision * exp);
                    pow.truncate_precision(prec);
                    Some(pow)
                }
            }
        } else {
            None
        }
    }

    pub fn is_zero(&self) -> bool {
        self.value == BigInt::ZERO
    }

    pub fn is_negative(&self) -> bool {
        self.value.sign() == Sign::Minus
    }

    /// Make the precision not higher than `prec`
    fn truncate_precision(&mut self, prec: u32) {
        if self.precision > prec {
            self.value /= TEN.pow(self.precision - prec);
            self.precision = prec;
        }
    }

    /// Drops the fractional part of the value
    fn value_truncated(&self) -> BigInt {
        &self.value / self.scaling()
    }

    /// The scaling factor needed to convert the fractional value to a float
    fn scaling(&self) -> BigInt {
        TEN.pow(self.precision)
    }

    pub fn to_u32_saturating(&self) -> u32 {
        if self.is_negative() {
            return 0;
        }
        let val = self.value_truncated();
        u32::try_from(val).unwrap_or(u32::MAX)
    }
}

/// Re-scale the values to have the same precision (max of both)
fn unify_precision<'a, 'b>(lhs: &'a Fixed, rhs: &'b Fixed) -> (Cow<'a, BigInt>, Cow<'b, BigInt>) {
    use std::cmp::Ordering::*;
    match lhs.precision.cmp(&rhs.precision) {
        Less => (
            Cow::Owned(&lhs.value * TEN.pow(rhs.precision - lhs.precision)),
            Cow::Borrowed(&rhs.value),
        ),
        Equal => (Cow::Borrowed(&lhs.value), Cow::Borrowed(&rhs.value)),
        Greater => (
            Cow::Borrowed(&lhs.value),
            Cow::Owned(&rhs.value * TEN.pow(lhs.precision - rhs.precision)),
        ),
    }
}

macro_rules! impl_op {
    ( $trait:tt, $method:tt ) => {
        impl $trait<&Fixed> for &Fixed {
            type Output = Fixed;

            /// Apply the operator. The scale of the result is equal to the max scale of both operands.
            fn $method(self, rhs: &Fixed) -> Self::Output {
                let (a, b) = unify_precision(self, rhs);
                Fixed::new(
                    a.as_ref().$method(b.as_ref()),
                    self.precision.max(rhs.precision),
                )
            }
        }
    };
}

impl_op!(Add, add);
impl_op!(Sub, sub);

impl Mul<&Fixed> for &Fixed {
    type Output = Fixed;

    /// Multiply two numbers with the precision min(a+b,max(scale,a,b))
    fn mul(self, rhs: &Fixed) -> Self::Output {
        let mut res = Fixed::new(&self.value * &rhs.value, self.precision + rhs.precision);
        let prec = unsafe { PRECISION }
            .max(self.precision)
            .max(rhs.precision)
            .min(self.precision + rhs.precision);
        res.truncate_precision(prec);
        res
    }
}

impl Div<&Fixed> for &Fixed {
    type Output = Fixed;

    /// Divide two numbers. The scale of the result is equal to scale.
    fn div(self, rhs: &Fixed) -> Self::Output {
        let prec = unsafe { PRECISION };
        let scaling = TEN.pow(prec);
        let (a, b) = unify_precision(self, rhs);
        Fixed::new(a.as_ref() * &scaling / b.as_ref(), prec)
    }
}

impl Rem<&Fixed> for &Fixed {
    type Output = Fixed;

    /// Calculates reminder defined as a-(a/b)*b (or `Sr dlr/ Lr*-` in dc), where
    /// "Remaindering is equivalent to 1) Computing a/b to current scale,
    /// and 2) Using the result of step 1 to calculate a-(a/b)*b to scale max(scale+scale(b),scale(a))."
    /// as described in the dc manual.
    fn rem(self, rhs: &Fixed) -> Self::Output {
        let div = self / rhs;

        let mut mul = Fixed::new(&div.value * &rhs.value, div.precision + rhs.precision);
        let prec = (unsafe { PRECISION } + rhs.precision).max(self.precision);
        mul.truncate_precision(prec);

        self - &mul
    }
}

impl Neg for Fixed {
    type Output = Fixed;

    fn neg(self) -> Self::Output {
        Fixed {
            value: self.value.neg(),
            precision: self.precision,
        }
    }
}

impl PartialEq for Fixed {
    fn eq(&self, other: &Self) -> bool {
        let (a, b) = unify_precision(self, other);
        a == b
    }
}

impl PartialOrd for Fixed {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let (a, b) = unify_precision(self, other);
        a.partial_cmp(&b)
    }
}

impl std::fmt::Display for Fixed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let radix = unsafe { OUTPUT_RADIX };
        write!(f, "{}", self.to_string_radix(radix))
    }
}

impl Default for Fixed {
    fn default() -> Self {
        Fixed {
            value: BigInt::ZERO,
            precision: 0,
        }
    }
}

fn count_digits(mut n: BigInt) -> u32 {
    debug_assert!(n >= BigInt::ZERO);
    if n == BigInt::ZERO {
        return 1;
    }
    // calculates floor(log10(n))
    let mut count = 0;
    while n != BigInt::ZERO {
        n /= 10;
        count += 1;
    }
    count
}

fn int_to_string(mut x: BigInt, radix: u32) -> String {
    debug_assert!(x >= BigInt::ZERO);
    if radix == 10 {
        return x.to_string();
    }
    // https://stackoverflow.com/a/50278316
    let mut acc = String::new();
    loop {
        let d = u32::try_from(&x % radix).unwrap();
        acc.push(char_from_digit(d, radix));
        x /= radix;
        if x == BigInt::ZERO {
            break;
        }
    }
    unsafe {
        acc.as_mut_vec().reverse();
    }
    acc
}

fn frac_to_string(mut x: BigInt, radix: u32, digits: u32) -> String {
    debug_assert!(x >= BigInt::ZERO);
    if radix == 10 {
        return format!("{:0>prec$}", x, prec = digits as usize);
    }
    // https://stackoverflow.com/a/20651123
    // https://www.electronics-tutorials.ws/binary/binary-fractions.html
    let base = TEN.pow(digits);
    let mut done = base.clone();
    let mut acc = String::new();
    while x != BigInt::ZERO && done > BigInt::ZERO {
        x *= radix;
        let d = u32::try_from(&x / &base).unwrap();
        acc.push(char_from_digit(d, radix));
        x %= &base;
        done /= radix;
    }
    acc
}

/// Produces an uppercase char representation of a digit
fn char_from_digit(n: u32, radix: u32) -> char {
    std::char::from_digit(n, radix)
        .unwrap()
        .to_ascii_uppercase()
}

/// Convert string byte to digit in hexadecimal notation, all other values are zeros
fn to_digit(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::Fixed;
    use crate::PRECISION;
    use test_case::test_case;

    #[test_case("0"; "zero")]
    #[test_case("0.0"; "middle dot")]
    #[test_case(".0"; "dot before")]
    #[test_case("0."; "dot after")]
    #[test_case("000000"; "multiple")]
    #[test_case("000000.000"; "multiple with middle dot")]
    fn parse_zeros(s: &str) {
        let val = Fixed::parse(s.as_bytes(), 10);
        assert_eq!("0", val.to_string())
    }

    #[test_case("10000")]
    #[test_case("123456789")]
    #[test_case("12345678.9")]
    #[test_case("123456.789")]
    #[test_case("12.3456789")]
    #[test_case("1.23456789")]
    #[test_case(".12345678")]
    #[test_case(".00000012")]
    fn parse_and_print(s: &str) {
        let val = Fixed::parse(s.as_bytes(), 10);
        assert_eq!(s, val.to_string())
    }

    #[test_case("2", "2", "4")]
    #[test_case("0.1", "0.2", "0.3")]
    #[test_case("1.111", "0.02", "1.131")]
    #[test_case("1234", "0.000056", "1234.000056")]
    #[test_case("0.000056", "1234", "1234.000056")]
    #[test_case("0.3", "20", "20.3")]
    #[test_case("20", "0.3", "20.3")]
    fn add(lhs: &str, rhs: &str, expected: &str) {
        let lhs = Fixed::parse(lhs.as_bytes(), 10);
        let rhs = Fixed::parse(rhs.as_bytes(), 10);
        let expected = Fixed::parse(expected.as_bytes(), 10);
        assert_eq!(&lhs + &rhs, expected)
    }

    #[test_case("2", "2", "4")]
    #[test_case("0.3", "20", "6")]
    #[test_case("20", "0.3", "6")]
    #[test_case("0.01", "0.003", "0.00003")]
    fn mul(lhs: &str, rhs: &str, expected: &str) {
        let lhs = Fixed::parse(lhs.as_bytes(), 10);
        let rhs = Fixed::parse(rhs.as_bytes(), 10);
        let expected = Fixed::parse(expected.as_bytes(), 10);
        assert_eq!(&lhs * &rhs, expected)
    }

    #[test_case("84", "2", "42")]
    #[test_case("20", "0.5", "40")]
    #[test_case("0.5", "20", "0.025")]
    #[test_case("0.03", "0.0001", "300")]
    #[test_case("0.00005", "20", "0.0000025")]
    #[test_case("20", "0.00005", "400000")]
    #[test_case("13.34", "7.892", "1.69031931")]
    #[test_case("7.892", "13.34", "0.59160419")]
    fn div(lhs: &str, rhs: &str, expected: &str) {
        unsafe { PRECISION = 8 }

        let lhs = Fixed::parse(lhs.as_bytes(), 10);
        let rhs = Fixed::parse(rhs.as_bytes(), 10);
        let expected = Fixed::parse(expected.as_bytes(), 10);
        assert_eq!(&lhs / &rhs, expected)
    }

    #[test_case("2", "2", "0")]
    #[test_case("3", "2", "1")]
    #[test_case("53", "22", "9")]
    #[test_case("31", "12", "7")]
    #[test_case("31.234", "12", "7.234")]
    fn rem(lhs: &str, rhs: &str, expected: &str) {
        unsafe { PRECISION = 0 }

        let lhs = Fixed::parse(lhs.as_bytes(), 10);
        let rhs = Fixed::parse(rhs.as_bytes(), 10);
        let expected = Fixed::parse(expected.as_bytes(), 10);
        assert_eq!(&lhs % &rhs, expected)
    }

    #[test_case("3.1415", "0", "1")]
    #[test_case("3.1415", "1", "3.1415")]
    #[test_case("5", "2", "25")]
    fn pow(lhs: &str, rhs: &str, expected: &str) {
        unsafe { PRECISION = 0 }

        let lhs = Fixed::parse(lhs.as_bytes(), 10);
        let rhs = Fixed::parse(rhs.as_bytes(), 10);
        let expected = Fixed::parse(expected.as_bytes(), 10);
        assert_eq!(lhs.checked_pow(&rhs).unwrap(), expected)
    }

    #[test_case("0", "0")]
    #[test_case("4", "2")]
    #[test_case("0.04", "0.2")]
    #[test_case("0.0004", "0.02")]
    fn sqrt(val: &str, expected: &str) {
        unsafe { PRECISION = 4 }
        let val = Fixed::parse(val.as_bytes(), 10);
        let expected = Fixed::parse(expected.as_bytes(), 10);
        assert_eq!(val.sqrt(), expected)
    }
}
