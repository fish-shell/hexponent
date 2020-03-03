#![deny(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::dbg_macro)]

//! # Hexponent
//!
//! Hexponent is a hexadecimal literal parser for Rust based on the C11
//! specification section [6.4.4.2](http://port70.net/~nsz/c/c11/n1570.html#6.4.4.2).
//!
//! ```rust
//! use hexponent::FloatLiteral;
//! let float_repr: FloatLiteral = "0x3.4".parse().unwrap();
//! let value = float_repr.convert::<f32>().inner();
//! assert_eq!(value, 3.25);
//! ```
//!
//! ## Features
//! - No dependencies
//! - Faster, non-UTF-8 parser
//! - Precision warnings
//!
//! ## Differences from the specification
//! There are two places where hexponent differs from the C11 specificaiton.
//! - An exponent is not required. (`0x1.2` is allowed)
//! - `floating-suffix` is *not* parsed. (`0x1p4l` is not allowed)

use std::fmt;

mod parse_utils;
use parse_utils::*;

mod fpformat;
pub use fpformat::FPFormat;

#[derive(Debug)]
/// Indicates the preicsision of a conversion
pub enum ConversionResult<T> {
    /// The conversion was precise and the result represents the original exactly.
    Precise(T),

    // TODO: I should be able to calculate how imprecise the conversion is too,
    // which might be useful. This also might allow some subnormal numbers to be
    // returned as precise results.
    /// The conversion was imprecise and the result is as close to the original
    /// as possible.
    Imprecise(T),
}

impl<T> ConversionResult<T> {
    /// Convert the result to it's contained type.
    pub fn inner(self) -> T {
        match self {
            ConversionResult::Precise(f) => f,
            ConversionResult::Imprecise(f) => f,
        }
    }
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

/// Represents a floating point literal
///
/// This struct is a representation of the text, that can be used to convert to
/// both single- and double-precision floats.
/// 
/// `FloatLiteral` is not `Copy`-able because it contains a vector of the
/// digits from the source data.
#[derive(Debug, Clone)]
pub struct FloatLiteral {
    is_positive: bool,
    digits: Vec<u8>,
    decimal_offset: i32,
    exponent: i32,
}

impl FloatLiteral {
    /// Convert the `self` to an `f32` or `f64` and return the precision of the
    /// conversion.
    pub fn convert<F: FPFormat>(self) -> ConversionResult<F> {
        F::from_literal(self)
    }

    /// Parse a slice of bytes into a `FloatLiteral`.
    ///
    /// This is based on hexadecimal floating constants in the C11 specification,
    /// section [6.4.4.2](http://port70.net/~nsz/c/c11/n1570.html#6.4.4.2).
    pub fn from_bytes(data: &[u8]) -> Result<FloatLiteral, ParseError> {
        let (is_positive, data) = match data.get(0) {
            Some(b'+') => (true, &data[1..]),
            Some(b'-') => (false, &data[1..]),
            _ => (true, data),
        };

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
                // TODO: Maybe make this uft8 conversion unchecked. It should be
                // good, but I also don't want unsafe code.
                let exponent: i32 =
                    std::str::from_utf8(&data[..sign_offset + exponent_digits_offset])
                        .expect("exponent did not contain valid utf-8")
                        .parse()?;

                (exponent, &data[sign_offset + exponent_digits_offset..])
            }
            _ => (0, data),
        };

        if !data.is_empty() {
            return Err(ParseError::ExtraData);
        }

        let mut raw_digits = ipart.to_vec();
        raw_digits.extend_from_slice(fpart);

        let first_digit = raw_digits.iter().position(|&d| d != b'0');

        let (digits, decimal_offset) = if let Some(first_digit) = first_digit {
            // Unwrap is safe because there is at least one digit.
            let last_digit = raw_digits.iter().rposition(|&d| d != b'0').unwrap();
            let decimal_offset = (ipart.len() as i32) - (first_digit as i32);

            (
                raw_digits[first_digit..=last_digit].to_vec(),
                decimal_offset,
            )
        } else {
            (Vec::new(), 0)
        };

        Ok(FloatLiteral {
            is_positive,
            digits,
            decimal_offset,
            exponent,
        })
    }
}

impl std::str::FromStr for FloatLiteral {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<FloatLiteral, ParseError> {
        FloatLiteral::from_bytes(s.as_bytes())
    }
}

impl Into<f32> for FloatLiteral {
    fn into(self) -> f32 {
        self.convert().inner()
    }
}

impl Into<f64> for FloatLiteral {
    fn into(self) -> f64 {
        self.convert().inner()
    }
}

#[cfg(test)]
mod tests;
