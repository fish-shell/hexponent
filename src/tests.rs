use crate::{FloatLiteral, ParseError};

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
    let float_repr = s.parse::<FloatLiteral>().unwrap();
    let float_result: f32 = float_repr.into();
    assert_eq_float!(float_result, result);
}

fn test_parse_error(s: &str, error: ParseError) {
    assert_eq!(s.parse::<FloatLiteral>().unwrap_err(), error);
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

    test_float("0x1p-8", 0.003_906_25);
    test_float("0x1p-4", 0.0625);
    test_float("0x1p0", 1.0);
    test_float("0x1p4", 16.0);
    test_float("0x1p8", 256.0);

    test_float("0x0.01p8", 1.0);
    test_float("0x0.1p4", 1.0);
    test_float("0x1p0", 1.0);
    test_float("0x10p-4", 1.0);
    test_float("0x100p-8", 1.0);
}

#[test]
fn test_overflow_underflow() {
    test_float("0x1p1000", std::f32::INFINITY);
    test_float("-0x1p1000", std::f32::NEG_INFINITY);

    // These two are technically wrong, but are correct enough. They should
    // acually return subnormal numbers, but i have not implemented that
    // yet.
    test_float("0x1p-128", 0.0);
    test_float("-0x1p-128", -0.0);

    // These acually should underflow to zero.
    test_float("0x1p-1000", 0.0);
    test_float("-0x1p-1000", -0.0);
}

#[test]
fn rcc_tests() {
    test_float("0x.ep0", 0.875);
    test_float("0x.ep-0", 0.875);
    test_float("0xe.p-4", 0.875);
    test_float("0xep-4", 0.875);

    // Hexf crashes on this one.
    "0x.000000000000000000102".parse::<FloatLiteral>().unwrap();
}

#[test]
fn test_incomplete() {
    test_parse_error("", ParseError::MissingPrefix);
    test_parse_error("-", ParseError::MissingPrefix);
    test_parse_error("+", ParseError::MissingPrefix);
    test_parse_error("-3.2", ParseError::MissingPrefix);
    test_parse_error("0x", ParseError::MissingDigits);
    test_parse_error("-0x", ParseError::MissingDigits);
    test_parse_error("+0x", ParseError::MissingDigits);
    test_parse_error("0x.", ParseError::MissingDigits);
    test_parse_error("0xp", ParseError::MissingDigits);
    test_parse_error("0x.p1", ParseError::MissingDigits);
    test_parse_error("0x1p", ParseError::MissingExponent);
    test_parse_error("0x1p+", ParseError::MissingExponent);
    test_parse_error("0x1p-", ParseError::MissingExponent);
    test_parse_error("0x1p10000000000", ParseError::ExponentOverflow);
    test_parse_error("0x1p-10000000000", ParseError::ExponentOverflow);
    test_parse_error("0xbaddata", ParseError::ExtraData);
}

#[test]
fn test_fuzzer_finds() {
    // Found by Byter on 2020-02-24
    "0X.0000002".parse::<FloatLiteral>().unwrap();
}

#[test]
fn test_zero_trimming() {
    test_float("0x0.0000000001p+40", 1.0);
    test_float("0x10000000000p-40", 1.0);

    // Right now these can only be tested to not crash because my rounding is
    // incorrect.
    "0x10000000000".parse::<FloatLiteral>().unwrap();
    "0x.0000000001".parse::<FloatLiteral>().unwrap();
}
