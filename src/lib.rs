#![deny(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::dbg_macro)]
#![cfg_attr(not(feature = "std"), no_std)]

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
//! Hexponent has a minimum supported rust version of 1.34.
//!
//! ## Features
//! - No dependencies
//! - Non-UTF-8 parser
//! - Precision warnings
//! - `no_std` support (MSRV 1.36.0)
//!
//! ## Differences from the specification
//! There are two places where hexponent differs from the C11 specificaiton.
//! - An exponent is not required. (`0x1.2` is allowed)
//! - `floating-suffix` is *not* parsed. (`0x1p4l` is not allowed)
//!
//! ## `no_std` support
//! `no_std` support can be enabled by disabling the default `std` feature for
//! hexponent in your `Cargo.toml`.
//! ```toml
//! hexponent = {version = "0.2", default-features = false}
//! ```
//! `no_std` support is only possible in rustc version 1.36.0 and higher.
//!
//! Disabling the `std` feature currently only disables the `std::error::Error`
//! implementation for `ParseError`.

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use core::fmt;

mod fpformat;
pub use fpformat::FPFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Indicates the precision of a conversion
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

    /// Return whether this result is precise.
    pub fn is_precise(&self) -> bool {
        matches!(self, ConversionResult::Precise(_))
    }

    /// Return whether this result is imprecise.
    pub fn is_imprecise(&self) -> bool {
        matches!(self, ConversionResult::Imprecise(_))
    }
}

/// Error type for parsing hexadecimal literals.
///
/// See the [`ParseErrorKind`](enum.ParseErrorKind.html) documentation for more
/// details about the kinds of errors and examples.
///
/// `ParseError` only implements `std::error::Error` when the `std` feature is
/// enabled.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ParseError {
    /// Kind of error
    pub kind: ParseErrorKind,
    /// Approximate index of the error in the source data. This will always be
    /// an index to the source, except for when something is expected and
    /// nothing is found, in this case, `index` will be the length of the input.
    pub index: usize,
}

/// Kind of parsing error.
///
/// Used in [`ParseError`](struct.ParseError.html)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParseErrorKind {
    /// No prefix was found. Hexadecimal literals must start with a "0x" or "0X"
    /// prefix.
    ///
    /// Example: `0.F`
    MissingPrefix,
    /// No digits were found. Hexadecimals literals must have digits before or
    /// after the decimal point.
    ///
    /// Example: `0x.` `0x.p1`
    MissingDigits,
    /// Hexadecimal literals with a "p" or "P" to indicate an float must have
    /// an exponent.
    ///
    /// Example: `0xb.0p` `0x1p-`
    MissingExponent,
    /// The exponent of a hexidecimal literal must fit into a signed 32-bit
    /// integer.
    ///
    /// Example: `0x1p3000000000`
    ExponentOverflow,
}

impl ParseErrorKind {
    fn at(self, index: usize) -> ParseError {
        ParseError { kind: self, index }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            ParseErrorKind::MissingPrefix => write!(f, "literal must have hex prefix"),
            ParseErrorKind::MissingDigits => write!(f, "literal must have digits"),
            ParseErrorKind::MissingExponent => write!(f, "exponent not present"),
            ParseErrorKind::ExponentOverflow => write!(f, "exponent too large to fit in integer"),
        }
    }
}

#[cfg(feature = "std")]
/// Only available with the `std` feature.
impl std::error::Error for ParseError {}

use std::iter::{Fuse, Peekable};

/// An iterator that counts the number of chars consumed.
pub struct CharsIterator<Chars>
where
    Chars: Iterator<Item = char>,
{
    chars: Peekable<Fuse<Chars>>,
    consumed: usize,
}

impl<Chars> CharsIterator<Chars>
where
    Chars: Iterator<Item = char>,
{
    /// Get the current char, or \0.
    fn current(&mut self) -> char {
        self.peek().unwrap_or('\0')
    }

    /// Get the current char, or None.
    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    /// Get the next char, incrementing self.consumed.
    fn next(&mut self) -> Option<char> {
        let res = self.chars.next();
        if res.is_some() {
            self.consumed += 1;
        }
        res
    }

    /// Consume a sequence of hex digits and return it as a sequence of u8s.
    /// The returned values are integers, not ascii characters.
    fn consume_hex_digits(&mut self) -> Vec<u8> {
        let mut digits = Vec::new();
        while let Some(digit) = self.peek().and_then(|c| c.to_digit(16)) {
            digits.push(digit as u8);
            self.next();
        }
        digits
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
    // These are the values of the digits, not the digits in ascii form.
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

    /// Helper used by the tests.
    #[cfg(test)]
    pub fn create(is_positive: bool, digits: Vec<u8>, decimal_offset: i32, exponent: i32) -> Self {
        FloatLiteral {
            is_positive,
            digits,
            decimal_offset,
            exponent,
        }
    }

    /// Parse a sequence of chars into a `FloatLiteral`.
    ///
    /// This is based on hexadecimal floating constants in the C11 specification,
    /// section [6.4.4.2](http://port70.net/~nsz/c/c11/n1570.html#6.4.4.2).
    pub fn from_chars<Chars>(
        input: Chars,
        decimal_sep: char,
        out_consumed: &mut usize,
    ) -> Result<FloatLiteral, ParseError>
    where
        Chars: Iterator<Item = char> + Clone,
    {
        let mut data = CharsIterator {
            chars: input.fuse().peekable(),
            consumed: 0,
        };

        let is_positive = match data.peek() {
            Some('+') => {
                data.next();
                true
            }
            Some('-') => {
                data.next();
                false
            }
            _ => true,
        };

        // Parse 0x or 0X prefix.
        let prefix_start = data.consumed;
        if data.current() != '0' {
            return Err(ParseErrorKind::MissingPrefix.at(prefix_start));
        }
        data.next();
        if data.current() != 'x' && data.current() != 'X' {
            return Err(ParseErrorKind::MissingPrefix.at(prefix_start));
        }
        data.next();

        let ipart: Vec<u8> = data.consume_hex_digits();
        let ipart_len = ipart.len();

        let fpart: Vec<u8> = if data.current() == decimal_sep {
            data.next();
            data.consume_hex_digits()
        } else {
            Vec::new()
        };

        // Must have digits before or after the decimal point.
        if fpart.is_empty() && ipart.is_empty() {
            return Err(ParseErrorKind::MissingDigits.at(data.consumed));
        }

        let mut exponent = 0;
        if data.current() == 'p' || data.current() == 'P' {
            data.next();

            let exponent_start = data.consumed;
            let mut exponent_str = String::new();
            match data.current() {
                '+' => {
                    data.next();
                }
                '-' => {
                    data.next();
                    exponent_str.push('-');
                }
                _ => {}
            };

            // Collect the exponent into a string, optionally with a sign, and then use Rust's parsing.
            while data.current().is_ascii_digit() {
                exponent_str.push(data.next().unwrap());
            }

            if exponent_str.is_empty() || exponent_str == "-" {
                return Err(ParseErrorKind::MissingExponent.at(exponent_start));
            }

            exponent = exponent_str
                .parse()
                .map_err(|_| ParseErrorKind::ExponentOverflow.at(exponent_start))?;
        }

        let mut raw_digits = ipart;
        raw_digits.extend_from_slice(&fpart);

        let first_digit = raw_digits.iter().position(|&d| d != 0);
        let (digits, decimal_offset) = if let Some(first_digit) = first_digit {
            // Unwrap is safe because there is at least one digit.
            let last_digit = raw_digits.iter().rposition(|&d| d != 0).unwrap();
            let decimal_offset = (ipart_len as i32) - (first_digit as i32);

            // Trim off the leading zeros
            raw_digits.truncate(last_digit + 1);
            // Trim off the trailing zeros
            raw_digits.drain(..first_digit);

            (raw_digits, decimal_offset)
        } else {
            (Vec::new(), 0)
        };

        *out_consumed = data.consumed;
        Ok(FloatLiteral {
            is_positive,
            digits,
            decimal_offset,
            exponent,
        })
    }
}

/// Parse a hex float from a sequence of Chars with the given decimal separator.
/// Return by reference the number of chars consumed.
pub fn parse_hex_float<Chars>(
    input: Chars,
    decimal_sep: char,
    out_consumed: &mut usize,
) -> Result<f64, ParseError>
where
    Chars: Iterator<Item = char> + Clone,
{
    FloatLiteral::from_chars(input, decimal_sep, out_consumed).map(|f| f.convert().inner())
}

impl core::str::FromStr for FloatLiteral {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<FloatLiteral, ParseError> {
        FloatLiteral::from_chars(s.chars(), '.', &mut 0)
    }
}

impl From<FloatLiteral> for f32 {
    fn from(literal: FloatLiteral) -> f32 {
        literal.convert().inner()
    }
}

impl From<FloatLiteral> for f64 {
    fn from(literal: FloatLiteral) -> f64 {
        literal.convert().inner()
    }
}

#[cfg(test)]
mod tests;
