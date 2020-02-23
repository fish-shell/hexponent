#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Hexponent
//!
//! This crate is used for parsing hexadecimal floating-point literals.

use std::fmt;

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
            return 0.0;
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

        let final_exponent = exponent_offset + self.exponent;

        // Check for underflows
        if final_exponent < std::f32::MIN_EXP - 1 {
            // TODO: Add a warning for underflow.
            // TODO: Implement subnormal numbers.
            return if self.is_positive { 0.0 } else { -0.0 };
        }

        // Check for overflows
        if final_exponent > std::f32::MAX_EXP {
            // TODO: Add a warning for overflow.
            return if self.is_positive {
                std::f32::INFINITY
            } else {
                std::f32::NEG_INFINITY
            };
        }

        let exponent_result: u32 =
            ((final_exponent + EXPONENT_BIAS as i32) as u32) << MANTISSA_BITS;

        let sign_result: u32 = (!self.is_positive as u32) << (MANTISSA_BITS + EXPONENT_BITS);

        f32::from_bits(
            sign_result | exponent_result | mantissa_result,
        )

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

/// Error type for parsing hexadecimal literals.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParseError {
    /// No prefix was found. Hexadecimal literals must start with a "0x" or "0X"
    /// prefix.
    MissingPrefix,
    /// No digits were found. Hexadecimals literals must have digits before or
    /// after the decimal point.
    MissingDigits,
    /// Hexadecimal literals with a "p" or "P" to indicate an exponent must have
    /// an exponent.
    MissingExponent,
    /// The exponent of a hexidecimal literal must fit into a signed 32-bit
    /// integer.
    ExponentOverflow,
    /// Extra bytes were found at the end of the hexadecimal literal.
    ExtraData,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::MissingPrefix => write!(f, "literal must have hex prefix"),
            ParseError::MissingDigits => write!(f, "literal must have digits"),
            ParseError::MissingExponent => write!(f, "exponent not present"),
            ParseError::ExponentOverflow => write!(f, "exponent too large to fit in integer"),
            ParseError::ExtraData => {
                write!(f, "extra bytes were found at the end of float literal")
            }
        }
    }
}

impl From<std::num::ParseIntError> for ParseError {
    fn from(_error: std::num::ParseIntError) -> ParseError {
        ParseError::ExponentOverflow
    }
}

/// Parse a slice of bytes into a `Float`.
///
/// This is based on hexadecimal floating constants in the C11 specification,
/// section [6.4.4.2](http://port70.net/~nsz/c/c11/n1570.html#6.4.4.2).
pub fn parse_float(data: &[u8]) -> Result<Float, ParseError> {
    let (is_positive, data) = consume_sign(data);

    let data = match data.get(0..2) {
        Some(b"0X") | Some(b"0x") => &data[2..],
        _ => return Err(ParseError::MissingPrefix),
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
        return Err(ParseError::MissingDigits);
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

            let sign_offset = match data.get(0) {
                Some(b'+') | Some(b'-') => 1,
                _ => 0,
            };

            let exponent_digits_offset = data[sign_offset..]
                .iter()
                .position(|&b| match b {
                    b'0'..=b'9' => false,
                    _ => true,
                })
                .unwrap_or_else(|| data[sign_offset..].len());

            if exponent_digits_offset == 0 {
                return Err(ParseError::MissingExponent);
            }

            // The exponent should always contain valid utf-8 beacuse it
            // consumes a sign, and base-10 digits.
            let exponent: i32 = std::str::from_utf8(&data[..sign_offset + exponent_digits_offset])
                .expect("exponent did not contain valid utf-8")
                .parse()?;

            (exponent, &data[sign_offset + exponent_digits_offset..])
        }
        _ => (0, data),
    };

    if !data.is_empty() {
        return Err(ParseError::ExtraData);
    }

    Ok(Float {
        is_positive,
        ipart: ipart.to_vec(),
        fpart: fpart.to_vec(),
        exponent,
    })
}

#[cfg(test)]
mod tests;
