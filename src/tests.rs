use crate::{parse_hex_float, ConversionResult, FloatLiteral, ParseError, ParseErrorKind};

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

macro_rules! assert_eq_double {
    ($left: expr, $right: expr) => {
        let left_val: f64 = $left;
        let right_val: f64 = $right;
        if left_val.to_bits() != right_val.to_bits() {
            panic!(
                r#"float assertion failed: `(left == right)`
left: `{:?}` (`{:016x}`)
right: `{:?}` (`{:016x}`)"#,
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
    let float_result: f32 = float_repr.convert().inner();
    assert_eq_float!(float_result, result);

    #[cfg(feature = "std")]
    {
        let libc_result = libc_funcs::string_to_f32(s.as_ref()).unwrap();
        assert_eq_float!(float_result, libc_result);
    }
}

fn test_double(s: &str, result: f64) {
    let float_repr = s.parse::<FloatLiteral>().unwrap();
    let double_result: f64 = float_repr.convert().inner();
    assert_eq_double!(double_result, result);

    #[cfg(feature = "std")]
    {
        let libc_result = libc_funcs::string_to_f64(s.as_ref()).unwrap();
        assert_eq_double!(double_result, libc_result);
    }
}

fn test_both(s: &str, float_result: f32) {
    let double_result = float_result as f64;
    test_float(s, float_result);
    test_double(s, double_result);
}

fn test_parse_error_kind(s: &str, error: ParseErrorKind) {
    assert_eq!(s.parse::<FloatLiteral>().unwrap_err().kind, error);
}

fn test_parse_error(s: &str, error: ParseError) {
    assert_eq!(s.parse::<FloatLiteral>().unwrap_err(), error);
}

fn test_imprecise(s: &str) {
    let float_literal = s.parse::<FloatLiteral>().unwrap();
    let conversion_result = float_literal.convert::<f32>();
    if let ConversionResult::Imprecise(_) = conversion_result {
        // Pass
    } else {
        panic!(
            "conversion from {:?} to float should have been imprecise (was {:?})",
            s, conversion_result
        );
    }
}

#[test]
fn test_zero() {
    test_both("0x0", 0.0);
    test_both("0x0.", 0.0);
    test_both("0x.0", 0.0);
    test_both("0x0.0", 0.0);
    test_both("0x0000.0000", 0.0);
}

#[test]
fn test_integers() {
    test_both("0x11", 17.0);
    test_both("0x21", 33.0);
    test_both("0x22", 34.0);

    test_both("0xDEAD", 57005.0);
    test_both("0xBEEF", 48879.0);
}

#[test]
fn test_fractions() {
    test_both("0x0.2", 0.125);
    test_both("0x0.4", 0.25);
    test_both("0x0.8", 0.5);
    test_both("0x0.c", 0.75);
    test_both("0x0.e", 0.875);
}

#[test]
fn test_exponents() {
    test_both("0x0.01", 0.003_906_25);
    test_both("0x0.1", 0.0625);
    test_both("0x1", 1.0);
    test_both("0x10", 16.0);
    test_both("0x100", 256.0);

    test_both("0x1p-8", 0.003_906_25);
    test_both("0x1p-4", 0.0625);
    test_both("0x1p0", 1.0);
    test_both("0x1p4", 16.0);
    test_both("0x1p8", 256.0);

    test_both("0x0.01p8", 1.0);
    test_both("0x0.1p4", 1.0);
    test_both("0x1p0", 1.0);
    test_both("0x10p-4", 1.0);
    test_both("0x100p-8", 1.0);
}

#[test]
fn test_overflow_underflow() {
    test_float("0x1p1000", core::f32::INFINITY);
    test_float("-0x1p1000", core::f32::NEG_INFINITY);
    test_float("0x1p-1000", 0.0);
    test_float("-0x1p-1000", -0.0);
}

#[test]
#[ignore]
fn test_subnormal() {
    // I haven't implemented subnormal numbers yet.
    test_float("0x1p-128", 0.0);
    test_float("-0x1p-128", -0.0);
}

#[test]
fn rcc_tests() {
    test_both("0x.ep0", 0.875);
    test_both("0x.ep-0", 0.875);
    test_both("0xe.p-4", 0.875);
    test_both("0xep-4", 0.875);

    // Hexf crashes on this one.
    "0x.000000000000000000102".parse::<FloatLiteral>().unwrap();
}

#[test]
fn test_incomplete() {
    test_parse_error_kind("", ParseErrorKind::MissingPrefix);
    test_parse_error_kind("-", ParseErrorKind::MissingPrefix);
    test_parse_error_kind("+", ParseErrorKind::MissingPrefix);
    test_parse_error_kind("-3.2", ParseErrorKind::MissingPrefix);
    test_parse_error_kind("0x", ParseErrorKind::MissingDigits);
    test_parse_error_kind("-0x", ParseErrorKind::MissingDigits);
    test_parse_error_kind("+0x", ParseErrorKind::MissingDigits);
    test_parse_error_kind("0x.", ParseErrorKind::MissingDigits);
    test_parse_error_kind("0xp", ParseErrorKind::MissingDigits);
    test_parse_error_kind("0x.p1", ParseErrorKind::MissingDigits);
    test_parse_error_kind("0x1p", ParseErrorKind::MissingExponent);
    test_parse_error_kind("0x1p+", ParseErrorKind::MissingExponent);
    test_parse_error_kind("0x1p-", ParseErrorKind::MissingExponent);
    test_parse_error_kind("0x1p10000000000", ParseErrorKind::ExponentOverflow);
    test_parse_error_kind("0x1p-10000000000", ParseErrorKind::ExponentOverflow);
}

#[test]
fn test_fuzzer_finds() {
    // Found by Byter on 2020-02-24
    "0X.0000002".parse::<FloatLiteral>().unwrap();

    // Found by Byter on 2020-02-29
    let literal = "0x3p127".parse::<FloatLiteral>().unwrap();
    assert!(!literal.convert::<f32>().inner().is_nan());
}

#[test]
fn test_zero_trimming() {
    test_both("0x0.0000000001p+40", 1.0);
    test_both("0x10000000000p-40", 1.0);

    // Right now these can only be tested to not crash because my rounding is
    // incorrect.
    "0x10000000000".parse::<FloatLiteral>().unwrap();
    "0x.0000000001".parse::<FloatLiteral>().unwrap();
}

#[test]
fn test_double_precision() {
    // test that float rounds and double doesn't
    test_float("0x1000000001", 68_719_480_000.0);
    test_double("0x1000000001", 68_719_476_737.0);
}

#[test]
fn test_imprecise_conversions() {
    test_imprecise("0x1p-10000"); // Underflow
    test_imprecise("0x123456789abcdef"); // Truncation
    test_imprecise("0x1p10000"); // Overflow
}

#[test]
fn test_error_indecies() {
    test_parse_error("0.F", ParseErrorKind::MissingPrefix.at(0));
    test_parse_error("0x.", ParseErrorKind::MissingDigits.at(3));
    test_parse_error("0x.p1", ParseErrorKind::MissingDigits.at(3));
    test_parse_error("0xb.0p", ParseErrorKind::MissingExponent.at(6));
    test_parse_error("0x1p-", ParseErrorKind::MissingExponent.at(4));
    test_parse_error("0x1p3000000000", ParseErrorKind::ExponentOverflow.at(4));
}

/// Helper which returns the components of a float as a tuple.
fn parse(s: &str) -> Result<(bool, u64, i32), ParseErrorKind> {
    match s.parse::<FloatLiteral>() {
        Ok(literal) => {
            // Convert mantissa to a u64.
            let mantissa = literal.digits.iter().fold(0_u64, |acc, &digit| {
                acc.checked_mul(16)
                    .unwrap()
                    .checked_add(digit as u64)
                    .unwrap()
            });
            Ok((!literal.is_positive, mantissa, literal.exponent))
        }
        Err(e) => Err(e.kind),
    }
}

#[test]
fn test_parse() {
    assert_eq!(parse(""), Err(ParseErrorKind::MissingPrefix));
    assert_eq!(parse(" "), Err(ParseErrorKind::MissingPrefix));
    assert_eq!(parse("3.14"), Err(ParseErrorKind::MissingPrefix));
    assert_eq!(parse("0x3.14"), Ok((false, 0x314, 0)));
    assert_eq!(parse("0x3.14fp+3"), Ok((false, 0x314f, 3)));
    assert_eq!(parse(" 0x3.14p+3"), Err(ParseErrorKind::MissingPrefix));
    assert_eq!(parse("0x3.14p+3 "), Ok((false, 0x314, 3)));
    assert_eq!(parse("+0x3.14fp+3"), Ok((false, 0x314f, 3)));
    assert_eq!(parse("-0x3.14fp+3"), Ok((true, 0x314f, 3)));
    assert_eq!(parse("0xAbC.p1"), Ok((false, 0xabc, 1)));
    assert_eq!(parse("0x0.7p1"), Ok((false, 0x7, 1)));
    assert_eq!(parse("0x.dEfP-1"), Ok((false, 0xdef, -1)));
    assert_eq!(parse("0x.p1"), Err(ParseErrorKind::MissingDigits));
    assert_eq!(parse("0x.P1"), Err(ParseErrorKind::MissingDigits));
    assert_eq!(parse("0xp1"), Err(ParseErrorKind::MissingDigits));
    assert_eq!(parse("0xP1"), Err(ParseErrorKind::MissingDigits));
    assert_eq!(parse("0x0p"), Err(ParseErrorKind::MissingExponent));
    assert_eq!(parse("0xp"), Err(ParseErrorKind::MissingDigits));
    assert_eq!(parse("0x.p"), Err(ParseErrorKind::MissingDigits));
    assert_eq!(parse("0x0p1"), Ok((false, 0, 1)));
    assert_eq!(parse("0x0P1"), Ok((false, 0, 1)));
    assert_eq!(parse("0x0.p1"), Ok((false, 0, 1)));
    assert_eq!(parse("0x0.P1"), Ok((false, 0, 1)));
    assert_eq!(parse("0x0.0p1"), Ok((false, 0, 1)));
    assert_eq!(parse("0x0.0P1"), Ok((false, 0, 1)));
    assert_eq!(parse("0x.0p1"), Ok((false, 0, 1)));
    assert_eq!(parse("0x.0P1"), Ok((false, 0, 1)));
    assert_eq!(parse("0x0p0"), Ok((false, 0, 0)));
    assert_eq!(parse("0x0.p999999999"), Ok((false, 0, 999999999)));
    assert_eq!(
        parse("0x0.p99999999999999999999999999999"),
        Err(ParseErrorKind::ExponentOverflow)
    );
    assert_eq!(
        parse("0x0.p-99999999999999999999999999999"),
        Err(ParseErrorKind::ExponentOverflow)
    );
    assert_eq!(
        parse("0x1.p99999999999999999999999999999"),
        Err(ParseErrorKind::ExponentOverflow)
    );
    assert_eq!(
        parse("0x1.p-99999999999999999999999999999"),
        Err(ParseErrorKind::ExponentOverflow)
    );
    assert_eq!(parse("0x4.00000000000000000000p55"), Ok((false, 4, 55)));
    assert_eq!(
        parse("0x4.00001000000000000000p55"),
        Ok((false, 0x400001, 55))
    );

    // issues
    // #11 (https://github.com/lifthrasiir/hexf/issues/11)
    assert_eq!(parse("0x1p-149"), parse("0x1.0p-149"));
}

#[test]
fn test_convert_hexf64() {
    fn convert_hexf64(negative: bool, mut mantissa: u64, exponent: i32) -> ConversionResult<f64> {
        let is_positive = !negative;
        // Convert mantissa to Vec<u8>.
        let mut digits = Vec::new();
        while mantissa > 0 {
            digits.push((mantissa % 16) as u8);
            mantissa /= 16;
        }
        digits.reverse();
        let decimal_offset = digits.len() as i32;
        let lit = FloatLiteral::create(is_positive, digits, decimal_offset, exponent);
        lit.convert()
    }

    use ConversionResult::{Imprecise, Precise};

    assert_eq!(convert_hexf64(false, 0, 0), Precise(0.0));
    assert_eq!(convert_hexf64(false, 1, 0), Precise(1.0));
    assert_eq!(convert_hexf64(false, 10, 0), Precise(10.0));
    assert_eq!(convert_hexf64(false, 10, 1), Precise(20.0));
    assert_eq!(convert_hexf64(false, 10, -1), Precise(5.0));
    assert_eq!(convert_hexf64(true, 0, 0), Precise(-0.0));
    assert_eq!(convert_hexf64(true, 1, 0), Precise(-1.0));

    // negative zeroes
    assert_eq!(convert_hexf64(false, 0, 0).inner().signum(), 1.0);
    assert_eq!(convert_hexf64(true, 0, 0).inner().signum(), -1.0);

    // normal truncation
    assert_eq!(
        convert_hexf64(false, 0x001f_ffff_ffff_ffff, 0),
        Precise(9007199254740991.0)
    );
    // This mantissas is "too big" but we report it as Precise.
    assert_eq!(
        convert_hexf64(false, 0x003f_ffff_ffff_ffff, 0),
        Precise(1.8014398509481982e16)
    );
    assert_eq!(
        convert_hexf64(false, 0xffff_ffff_ffff_f800, -11),
        Imprecise(9007199254740991.0)
    );
    assert_eq!(
        convert_hexf64(false, 0xffff_ffff_ffff_fc00, -11),
        Imprecise(9007199254740991.0)
    );

    // denormal truncation
    // TODO: denormals are not supported yet.
    //assert!(convert_hexf64(false, 0x000f_ffff_ffff_ffff, -1074).is_precise());
    //assert!(convert_hexf64(false, 0x001f_ffff_ffff_ffff, -1075).is_imprecise());
    //assert!(convert_hexf64(false, 0x001f_ffff_ffff_fffe, -1075).is_precise());
    //assert!(convert_hexf64(false, 0xffff_ffff_ffff_f800, -1086).is_imprecise());
    //assert!(convert_hexf64(false, 0xffff_ffff_ffff_f000, -1086).is_precise());

    // minimum
    //assert!(convert_hexf64(false, 0x0000_0000_0000_0001, -1074).is_precise());
    //assert!(convert_hexf64(false, 0x0000_0000_0000_0001, -1075).is_imprecise());
    //assert!(convert_hexf64(false, 0x0000_0000_0000_0002, -1075).is_precise());
    //assert!(convert_hexf64(false, 0x0000_0000_0000_0002, -1076).is_imprecise());
    //assert!(convert_hexf64(false, 0x0000_0000_0000_0003, -1075).is_imprecise());
    //assert!(convert_hexf64(false, 0x0000_0000_0000_0003, -1076).is_imprecise());
    //assert!(convert_hexf64(false, 0x8000_0000_0000_0000, -1137).is_precise());
    //assert!(convert_hexf64(false, 0x8000_0000_0000_0000, -1138).is_imprecise());

    // maximum
    assert_eq!(
        convert_hexf64(false, 0x001f_ffff_ffff_ffff, 971),
        Precise(f64::MAX)
    );
    assert_eq!(
        convert_hexf64(false, 0x003f_ffff_ffff_ffff, 971),
        Imprecise(f64::INFINITY)
    );
    assert_eq!(
        convert_hexf64(false, 0x003f_ffff_ffff_fffe, 971),
        Imprecise(f64::INFINITY)
    );
    assert_eq!(
        convert_hexf64(false, 0xffff_ffff_ffff_f800, 960),
        // TODO: this should be precise.
        Imprecise(f64::MAX)
    );
    assert_eq!(
        convert_hexf64(false, 0xffff_ffff_ffff_fc00, 960),
        Imprecise(f64::MAX)
    );
}

#[test]
fn test_consumed() {
    fn consumed(s: &str) -> usize {
        let mut res = 0;
        let comp = parse_hex_float(s.chars(), '.', &mut res);
        if comp.is_err() {
            usize::MAX
        } else {
            res
        }
    }
    assert_eq!(consumed("0x3.14p+3 "), 9);
    assert_eq!(consumed("0x3.14p+3    "), 9);
    assert_eq!(consumed("0x0p3    "), 5);
    assert_eq!(consumed("0x0000000p3    "), 11);
    assert_eq!(consumed("-0x0p3    "), 6);
}

#[cfg(feature = "std")]
mod libc_funcs {
    use std::ffi;

    // I had both of these functions checked over by jynelson
    #[allow(unsafe_code, dead_code)]
    pub fn f64_to_string(f: f64) -> Result<Vec<u8>, ()> {
        let mut dest = [0u8; 32];
        let format = ffi::CString::new("%a").unwrap();
        let number = f as libc::c_double;
        let check =
            unsafe { libc::snprintf(dest.as_mut_ptr() as *mut i8, 32, format.as_ptr(), number) };
        if check >= 0 && check < 32 {
            Ok(dest[..check as usize].to_vec())
        } else {
            Err(())
        }
    }

    #[allow(unsafe_code)]
    pub fn string_to_f32(string: &[u8]) -> Result<f32, ()> {
        let source = ffi::CString::new(string).unwrap();
        let format = ffi::CString::new("%a").unwrap();
        let mut dest: f32 = 0.0;
        let check = unsafe { libc::sscanf(source.as_ptr(), format.as_ptr(), &mut dest) };
        if check == 1 {
            Ok(dest)
        } else {
            Err(())
        }
    }

    #[allow(unsafe_code)]
    pub fn string_to_f64(string: &[u8]) -> Result<f64, ()> {
        let source = ffi::CString::new(string).unwrap();
        let format = ffi::CString::new("%la").unwrap();
        let mut dest: f64 = 0.0;
        let check = unsafe { libc::sscanf(source.as_ptr(), format.as_ptr(), &mut dest) };
        if check == 1 {
            Ok(dest)
        } else {
            Err(())
        }
    }
}
