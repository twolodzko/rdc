use crate::PRECISION;
use num_bigint::BigInt;
use std::{
    borrow::Cow,
    ops::{Add, Div, Mul, Neg, Rem, Sub},
};

#[derive(Debug, Clone)]
pub struct Fixed {
    value: BigInt,
    precision: u32,
}

impl Fixed {
    pub const ZERO: Fixed = Fixed {
        value: BigInt::ZERO,
        precision: 0,
    };

    fn new(value: BigInt, mut precision: u32) -> Fixed {
        if value == BigInt::ZERO {
            precision = 0
        }
        Fixed { value, precision }
    }

    pub fn sqrt(&self) -> Fixed {
        if self.value == BigInt::ZERO {
            return self.clone();
        }
        let prec = self.precision.max(unsafe { PRECISION });
        let scaling = BigInt::from(10).pow(prec);

        // the decimal number is represented by a fractional one x/n
        // where n is the scaling factor 10^p and p is the precission (number of decimal points)
        // and stored as an integer y=(x/n)*n
        // by the property of square root: sqrt(ab) = sqrt(a) * sqrt(b)
        // so if we want to keep the precision to be p, we need to take
        // sqrt(y * n) = sqrt((x/n) * n * n) = sqrt(x/n) * n
        // so the scaling factor remains unchanged

        let value = (&self.value * self.scaling() * scaling.pow(2)).sqrt() / self.scaling();
        Fixed::new(value, prec)
    }

    pub fn checked_pow(&self, rhs: &Fixed) -> Option<Fixed> {
        // keep integer part of exponent
        if let Ok(exp) = u32::try_from(rhs.value_truncated()) {
            if exp == 0 {
                return Some(Fixed {
                    value: BigInt::ONE,
                    precision: 0,
                });
            }
            if exp == 1 {
                return Some(self.clone());
            }
            let prec = unsafe { PRECISION }.max(self.precision);
            let scaling = BigInt::from(10).pow(prec);

            let mut pow = Fixed::new((&self.value * &scaling).pow(exp), self.precision + prec);
            if pow.precision > 0 {
                pow.precision *= exp;
            }
            pow.truncate_precision(prec);
            Some(pow)
        } else {
            None
        }
    }

    pub fn is_zero(&self) -> bool {
        self.value == BigInt::ZERO
    }

    pub fn is_negative(&self) -> bool {
        self.value < BigInt::ZERO
    }

    fn truncate_precision(&mut self, prec: u32) {
        if self.precision > prec {
            self.value /= BigInt::from(10).pow(self.precision - prec);
            self.precision = prec;
        }
    }

    /// Drops the fractional part of the value
    fn value_truncated(&self) -> BigInt {
        &self.value / self.scaling()
    }

    fn scaling(&self) -> BigInt {
        BigInt::from(10).pow(self.precision)
    }

    pub fn to_u32_saturating(&self) -> u32 {
        let val = self.value_truncated();
        u32::try_from(val).unwrap_or(u32::MAX)
    }
}

fn unify_precision<'a, 'b>(lhs: &'a Fixed, rhs: &'b Fixed) -> (Cow<'a, BigInt>, Cow<'b, BigInt>) {
    use std::cmp::Ordering::*;
    match lhs.precision.cmp(&rhs.precision) {
        Less => (
            Cow::Owned(&lhs.value * BigInt::from(10).pow(rhs.precision - lhs.precision)),
            Cow::Borrowed(&rhs.value),
        ),
        Equal => (Cow::Borrowed(&lhs.value), Cow::Borrowed(&rhs.value)),
        Greater => (
            Cow::Borrowed(&lhs.value),
            Cow::Owned(&rhs.value * BigInt::from(10).pow(lhs.precision - rhs.precision)),
        ),
    }
}

macro_rules! impl_op {
    ( $trait:tt, $method:tt ) => {
        impl $trait<&Fixed> for &Fixed {
            type Output = Fixed;

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

    fn mul(self, rhs: &Fixed) -> Self::Output {
        let mut res = Fixed::new(&self.value * &rhs.value, self.precision + rhs.precision);
        let prec = unsafe { PRECISION }.max(self.precision).max(rhs.precision);
        res.truncate_precision(prec);
        res
    }
}

impl Div<&Fixed> for &Fixed {
    type Output = Fixed;

    fn div(self, rhs: &Fixed) -> Self::Output {
        let prec = unsafe { PRECISION };
        let scaling = BigInt::from(10).pow(prec);
        let (a, b) = unify_precision(self, rhs);
        Fixed::new(a.as_ref() * &scaling / b.as_ref(), prec)
    }
}

impl Rem<&Fixed> for &Fixed {
    type Output = Fixed;

    fn rem(self, rhs: &Fixed) -> Self::Output {
        // same as: Sr dlr/ Lr*-
        // a - n * (a/n)
        let div = self / rhs;
        let mul = rhs * &div;
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
        &self.value * BigInt::from(10).pow(other.precision)
            == &other.value * BigInt::from(10).pow(self.precision)
    }
}

impl PartialOrd for Fixed {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let (a, b) = unify_precision(self, other);
        a.partial_cmp(&b)
    }
}

impl From<&[u8]> for Fixed {
    fn from(bytes: &[u8]) -> Self {
        let mut value = BigInt::ZERO;
        let mut precision = 0;
        let mut base = BigInt::ONE;
        let mut i = bytes.len();
        while i != 0 {
            i -= 1;
            match bytes[i] {
                b'.' => {
                    precision = bytes.len() - i - 1;
                    break;
                }
                b'0' => {}
                v => value += &base * (v - b'0'),
            }
            base *= 10;
        }
        while i != 0 {
            i -= 1;
            value += &base * (bytes[i] - b'0');
            base *= 10;
        }
        Fixed::new(value, precision as u32)
    }
}

impl std::fmt::Display for Fixed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.precision > 0 {
            let s = self.value.to_string();
            if s.len() < self.precision as usize {
                let n = self.precision as usize - s.len();
                let z = "0".repeat(n);
                write!(f, ".{}{}", z, s)
            } else {
                let (int, frac) = s.split_at(s.len() - self.precision as usize);
                write!(f, "{}.{}", int, frac)
            }
        } else {
            write!(f, "{}", self.value)
        }
    }
}

impl Default for Fixed {
    fn default() -> Self {
        Fixed::ZERO
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
        let val = Fixed::from(s.as_bytes());
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
        let val = Fixed::from(s.as_bytes());
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
        let lhs = Fixed::from(lhs.as_bytes());
        let rhs = Fixed::from(rhs.as_bytes());
        let expected = Fixed::from(expected.as_bytes());
        assert_eq!(&lhs + &rhs, expected)
    }

    #[test_case("2", "2", "4")]
    #[test_case("0.3", "20", "6")]
    #[test_case("20", "0.3", "6")]
    #[test_case("0.01", "0.003", "0.00003")]
    fn mul(lhs: &str, rhs: &str, expected: &str) {
        let lhs = Fixed::from(lhs.as_bytes());
        let rhs = Fixed::from(rhs.as_bytes());
        let expected = Fixed::from(expected.as_bytes());
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

        let lhs = Fixed::from(lhs.as_bytes());
        let rhs = Fixed::from(rhs.as_bytes());
        let expected = Fixed::from(expected.as_bytes());
        assert_eq!(&lhs / &rhs, expected)
    }

    #[test_case("2", "2", "0")]
    #[test_case("3", "2", "1")]
    #[test_case("53", "22", "9")]
    #[test_case("31", "12", "7")]
    #[test_case("31.234", "12", "7.234")]
    fn rem(lhs: &str, rhs: &str, expected: &str) {
        unsafe { PRECISION = 0 }

        let lhs = Fixed::from(lhs.as_bytes());
        let rhs = Fixed::from(rhs.as_bytes());
        let expected = Fixed::from(expected.as_bytes());
        assert_eq!(&lhs % &rhs, expected)
    }

    #[test_case("3.1415", "0", "1")]
    #[test_case("3.1415", "1", "3.1415")]
    #[test_case("5", "2", "25")]
    fn pow(lhs: &str, rhs: &str, expected: &str) {
        unsafe { PRECISION = 0 }

        let lhs = Fixed::from(lhs.as_bytes());
        let rhs = Fixed::from(rhs.as_bytes());
        let expected = Fixed::from(expected.as_bytes());
        assert_eq!(lhs.checked_pow(&rhs).unwrap(), expected)
    }

    #[test_case("0", "0")]
    #[test_case("4", "2")]
    #[test_case("0.04", "0.2")]
    #[test_case("0.0004", "0.02")]
    fn sqrt(val: &str, expected: &str) {
        unsafe { PRECISION = 4 }
        let val = Fixed::from(val.as_bytes());
        let expected = Fixed::from(expected.as_bytes());
        assert_eq!(val.sqrt(), expected)
    }
}
