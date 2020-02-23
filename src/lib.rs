#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Hexponent
//!
//! This crate is used for parsing hexadecimal floating-point literals.

use std::convert;
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

impl Into<f32> for Float {
    fn into(self) -> f32 {
        // This code should work for arbitrary values of the following
        // constants, but a more efficient method is available for single- and
        // double-precision. (doing the mantissa all at once)

        // Number of bits available to store the exponent
        const EXPONENT_BITS: u32 = 8;
        const EXPONENT_BIAS: u32 = 127;
        // Number of bits available to store the mantissa
        const MANTISSA_BITS: u32 = 23;

        // 4 bits for each digit of the ipart
        let mut exponent_offset: i32 = (self.ipart.len() as i32) * 4;

        let mut digits = self.ipart;
        // Reserve enough space for the fpart and padding zero
        digits.reserve(self.fpart.len() + 1);
        digits.extend_from_slice(&self.fpart);
        digits.push(b'0'); // Get rid of this hack

        // These parts are the digits all strung together and put into parts of
        // two digits. It doesn't matter where the (hexa)decimal point was
        // because it was accouned for in the exponent_offset.
        let mut parts = digits.chunks(8 / 4).map(|hex_digits| {
            u8::from_str_radix(std::str::from_utf8(hex_digits).unwrap(), 16).unwrap()
        });

        let mut mantissa_result = 0u32;
        // This variable is used to keep track of how many bits are used in the
        // mantissa. It is used to shift each part by the right amount so it can
        // be put into the mantissa_result correctly.
        let mut mantissa_bits_left: i32 = MANTISSA_BITS as i32;

        loop {
            // TODO: Get rid of this unwrap.
            let mut part = parts.next().unwrap();

            // Take off all leading zero digits.
            if part == 0 {
                exponent_offset -= 8;
            } else {
                // For the first non-zero digit, take of all leading zero bits,
                // and the first one. The first one is implied as part of the
                // IEEE754 Standard.
                let leading_zeros = part.leading_zeros();
                exponent_offset -= (leading_zeros + 1) as i32;
                part <<= leading_zeros + 1;
                mantissa_result |= (part as u32) << (mantissa_bits_left - 8);
                mantissa_bits_left -= 7 - leading_zeros as i32;
                break;
            }
        }

        for part in parts {
            // If there are no mantissa bits left, drop excess precision.
            if mantissa_bits_left <= 0 {
                break;
            }

            if mantissa_bits_left >= 8 {
                // The part needs to be shifted left to be in the correct
                // position.
                mantissa_result |= (part as u32) << (mantissa_bits_left - 8);
            } else {
                // The byte needs to be shifted right to be in the correct
                // position. This also shifts out the last parts of the byte
                // that does not fit.
                mantissa_result |= (part as u32) >> (8 - mantissa_bits_left);
            }
            mantissa_bits_left -= 8;
        }

        // Multiply self.exponent by four because it is the base-16 exponent.
        // It needs to be in base-2. 16^x = 2^4x
        let final_exponent = exponent_offset + 4 * self.exponent;

        let exponent_result: u32 =
            ((final_exponent + EXPONENT_BIAS as i32) as u32) << MANTISSA_BITS;

        let sign_result: u32 = (!self.is_positive as u32) << (MANTISSA_BITS + EXPONENT_BITS);

        f32::from_bits(sign_result | exponent_result | mantissa_result)
    }
}

impl convert::TryFrom<&[u8]> for Float {
    type Error = Box<dyn Error>;

    fn try_from(data: &[u8]) -> Result<Float, Box<dyn Error>> {
        parse_float(data)
    }
}

// // This should be trivial
// impl Into<f64> for Float {
//     impl into(self) -> f64 {

//     }
// }

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
        .position(|b| match b {
            b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' => false,
            _ => true,
        })
        .unwrap_or_else(|| data.len());

    data.split_at(i)
}

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
    let ipart: &[u8] = if let Some(first_nonzero_digit) = ipart.iter().position(|&d| d == b'0') {
        &ipart[first_nonzero_digit..]
    } else {
        &[]
    };

    // Trim trailing zeros
    let fpart: &[u8] = if let Some(last_nonzero_digit) = fpart.iter().rposition(|&d| d == b'0') {
        &fpart[..last_nonzero_digit]
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

    #[allow(clippy::float_cmp)]
    fn test_float(s: &str, result: f32) {
        let float_repr = parse_float(s.as_ref()).unwrap();
        let float_result: f32 = float_repr.into();
        assert_eq!(float_result, result);
    }

    #[test]
    fn math_tests() {
        test_float("0x0.8", 0.5);
        test_float("0x0.4", 0.25);

        // test_float("0x0.01", 0.003_906_25);
        // test_float("0x0.1", 0.0625);
        // test_float("0x1", 1.0);
        // test_float("0x10", 16.0);
        // test_float("0x100", 266.0);
    }
}
