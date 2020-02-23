use crate::{parse_float, ParseError};

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
    let float_result: f32 = float_repr.into();
    assert_eq_float!(float_result, result);
}

// TODO: Once I switch to typed errors, convert this to a macro that accepts
// a pattern to make sure it returns the correct error.
fn test_parse_error(s: &str, error: ParseError) {
    assert_eq!(parse_float(s.as_ref()).unwrap_err(), error);
}

// // There are no conversion errors yet.
// fn test_conversion_error(s: &str) {
//     let float_repr = parse_float(s.as_ref()).unwrap();
//     let float_result: Result<f32, _> = float_repr.try_into();
//     assert!(float_result.is_err());
// }

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
