#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Hexponent
//!
//! This crate is used for parsing hexadecimal floating-point literals.

use std::convert::TryInto;
use std::error::Error;

fn hex_digit_to_int(digit: u8) -> Option<u8> {
    match digit {
        b'0' => Some(0x0),
        b'1' => Some(0x1),
        b'2' => Some(0x2),
        b'3' => Some(0x3),
        b'4' => Some(0x4),
        b'5' => Some(0x5),
        b'6' => Some(0x6),
        b'7' => Some(0x7),
        b'8' => Some(0x8),
        b'9' => Some(0x9),
        b'a' | b'A' => Some(0xa),
        b'b' | b'B' => Some(0xb),
        b'c' | b'C' => Some(0xc),
        b'd' | b'D' => Some(0xd),
        b'e' | b'E' => Some(0xe),
        b'f' | b'F' => Some(0xf),
        _ => None,
    }
}

/// Represents a floating point literal
///
/// This struct is a representation of the text, it can be used to convert to
/// both single- and double-precision floats.
#[derive(Debug, Clone)]
pub struct Float {
    is_positive: bool,
    ipart: Vec<u8>,
    fpart: Vec<u8>,
    exponent: i32,
}

impl TryInto<f32> for Float {
    type Error = Box<dyn Error>;

    fn try_into(self) -> Result<f32, Box<dyn Error>> {
        // This code should work for arbitrary values of the following
        // constants
        const EXPONENT_BITS: u32 = 8;
        const MANTISSA_BITS: u32 = 23;

        // The spec always gives an exponent bias that follows this formula.
        const EXPONENT_BIAS: u32 = (1 << (EXPONENT_BITS - 1)) - 1;

        // 4 bits for each digit of the ipart
        let mut exponent_offset: i32 = (self.ipart.len() as i32) * 4;

        // All the digits together, it doesn't matter where the (hexa)decimal
        // point was because it was accounted for in the exponent_offset.
        let mut digits = self.ipart;
        digits.extend_from_slice(&self.fpart);

        // If there were all
        if digits.is_empty() {
            return Ok(0.0);
        }

        // This code is a work of art.
        let mut mantissa_result: u32 = 0;
        for (index, digit) in digits.iter().enumerate() {
            if index as u32 * 4 > MANTISSA_BITS {
                // TODO: Warn for excessive precision.
                break;
            }
            let mut digit_value = hex_digit_to_int(*digit).unwrap() as u32;
            digit_value <<= 32 - (index + 1) * 4;
            mantissa_result |= digit_value;
        }
        let leading_zeros = mantissa_result.leading_zeros();
        exponent_offset -= leading_zeros as i32 + 1;
        mantissa_result <<= leading_zeros + 1;
        mantissa_result >>= 32 - MANTISSA_BITS;

        // Multiply self.exponent by four because it is the base-16 exponent.
        // It needs to be in base-2. 16^x = 2^4x
        let final_exponent = exponent_offset + 4 * self.exponent;

        // Check for underflows
        if final_exponent < std::f32::MIN_EXP - 1 {
            // TODO: Add a warning for underflow.
            // TODO: Implement subnormal numbers.
            return Ok(if self.is_positive { 0.0 } else { -0.0 });
        }

        // Check for overflows
        if final_exponent > std::f32::MAX_EXP {
            // TODO: Add a warning for overflow.
            return Ok(if self.is_positive {
                std::f32::INFINITY
            } else {
                std::f32::NEG_INFINITY
            });
        }

        let exponent_result: u32 =
            ((final_exponent + EXPONENT_BIAS as i32) as u32) << MANTISSA_BITS;

        let sign_result: u32 = (!self.is_positive as u32) << (MANTISSA_BITS + EXPONENT_BITS);

        Ok(f32::from_bits(
            sign_result | exponent_result | mantissa_result,
        ))

        // // This might be a bit faster.
        // let mut final_result = !self.is_positive as u32;
        // final_result <<= EXPONENT_BITS;
        // final_result |= (final_exponent + EXPONENT_BIAS as i32) as u32;
        // final_result <<= MANTISSA_BITS;
        // final_result |= mantissa_result;
        // f32::from_bits(final_result)
    }
}

fn consume_sign(data: &[u8]) -> (bool, &[u8]) {
    match data.get(0) {
        Some(b'+') => (true, &data[1..]),
        Some(b'-') => (false, &data[1..]),
        _ => (true, data),
    }
}

fn consume_hex_digits(data: &[u8]) -> (&[u8], &[u8]) {
    let i = data
        .iter()
        .position(|&b| hex_digit_to_int(b).is_none())
        .unwrap_or_else(|| data.len());

    data.split_at(i)
}

/// Parse a slice of bytes into a `Float`.
pub fn parse_float(data: &[u8]) -> Result<Float, Box<dyn Error>> {
    let (is_positive, data) = consume_sign(data);

    let data = match data.get(0..2) {
        Some(b"0X") | Some(b"0x") => &data[2..],
        _ => return Err("Literal must begin with '0x'".into()),
    };

    let (ipart, data) = consume_hex_digits(data);

    let (fpart, data): (&[_], _) = if data.get(0) == Some(&b'.') {
        let (fpart, data) = consume_hex_digits(&data[1..]);
        (fpart, data)
    } else {
        (b"", data)
    };

    // Must have digits before or after the decimal point.
    if fpart.is_empty() && ipart.is_empty() {
        return Err("Not enough digits.".into());
    }

    // Trim leading zeros.
    let ipart: &[u8] = if let Some(first_digit) = ipart.iter().position(|&d| d != b'0') {
        &ipart[first_digit..]
    } else {
        &[]
    };

    // Trim trailing zeros
    let fpart: &[u8] = if let Some(last_digit) = fpart.iter().rposition(|&d| d != b'0') {
        &fpart[..=last_digit]
    } else {
        &[]
    };

    let (exponent, data) = match data.get(0) {
        Some(b'P') | Some(b'p') => {
            let data = &data[1..];

            let (is_positive, data) = consume_sign(data);
            let (hex_digits, data) = consume_hex_digits(data);

            if hex_digits.is_empty() {
                return Err("Exponent mut have digits.".into());
            }

            let mut value: i32 = 0;
            for digit in hex_digits {
                value <<= 4;
                // This unwrap should be safe because consume_hex_digits ensures
                // valid hex digits are present.
                value |= hex_digit_to_int(*digit).unwrap() as i32;
            }

            let signum = if is_positive { 1 } else { -1 };

            (value * signum, data)
        }
        _ => (0, data),
    };

    if !data.is_empty() {
        dbg!(data);
        return Err("Extra bytes at end of float".into());
    }

    Ok(Float {
        is_positive,
        ipart: ipart.to_vec(),
        fpart: fpart.to_vec(),
        exponent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // This macros serves two functions:
    // 1. It avoids the float_cmp clippy lint
    // 2. It is able to tell the difference between floats that are equal, but
    // are not the same. (ex: zero and negative zero)
    macro_rules! assert_eq_float {
        ($left: expr, $right: expr) => {
            let left_val: f32 = $left;
            let right_val: f32 = $right;
            if left_val.to_bits() != right_val.to_bits() {
                panic!(
                    r#"float assertion failed: `(left == right)`
left: `{:?}` (`{:08x}`)
right: `{:?}` (`{:08x}`)"#,
                    left_val,
                    left_val.to_bits(),
                    right_val,
                    right_val.to_bits()
                );
            }
        };
    }

    fn test_float(s: &str, result: f32) {
        let float_repr = parse_float(s.as_ref()).unwrap();
        let float_result: f32 = float_repr.try_into().unwrap();
        assert_eq_float!(float_result, result);
    }

    #[test]
    fn test_zero() {
        test_float("0x0", 0.0);
        test_float("0x0.", 0.0);
        test_float("0x.0", 0.0);
        test_float("0x0.0", 0.0);
        test_float("0x0000.0000", 0.0);
    }

    #[test]
    fn test_integers() {
        test_float("0x11", 17.0);
        test_float("0x21", 33.0);
        test_float("0x22", 34.0);

        test_float("0xDEAD", 57005.0);
        test_float("0xBEEF", 48879.0);
    }

    #[test]
    fn test_fractions() {
        test_float("0x0.2", 0.125);
        test_float("0x0.4", 0.25);
        test_float("0x0.8", 0.5);
        test_float("0x0.c", 0.75);
        test_float("0x0.e", 0.875);
    }

    #[test]
    fn test_exponents() {
        test_float("0x0.01", 0.003_906_25);
        test_float("0x0.1", 0.0625);
        test_float("0x1", 1.0);
        test_float("0x10", 16.0);
        test_float("0x100", 256.0);

        test_float("0x1p-2", 0.003_906_25);
        test_float("0x1p-1", 0.0625);
        test_float("0x1p0", 1.0);
        test_float("0x1p1", 16.0);
        test_float("0x1p2", 256.0);

        test_float("0x0.01p2", 1.0);
        test_float("0x0.1p1", 1.0);
        test_float("0x1p0", 1.0);
        test_float("0x10p-1", 1.0);
        test_float("0x100p-2", 1.0);
    }

    #[test]
    fn test_overflow_underflow() {
        test_float("0x1p1000", std::f32::INFINITY);
        test_float("-0x1p1000", std::f32::NEG_INFINITY);

        // These two are technically wrong, but are correct enough. They should
        // acually return subnormal numbers, but i have not implemented that
        // yet.
        test_float("0x1p-26", 0.0);
        test_float("-0x1p-26", -0.0);

        // These acually should underflow to zero.
        test_float("0x1p-1000", 0.0);
        test_float("-0x1p-1000", -0.0);
    }
}
