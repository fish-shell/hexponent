use crate::{ConversionResult, FloatLiteral};
use core::ops;

macro_rules! impl_fpformat {
    ($fp_type:ty, $bits_type:ty, $exponent_bits: literal, $mantissa_bits: literal, $from_bits: expr, $infinity: expr, $max_exp: expr, $min_exp: expr) => {
        impl FPFormat for $fp_type {
            fn from_literal(literal: FloatLiteral) -> ConversionResult<$fp_type> {
                const EXPONENT_BITS: u32 = $exponent_bits;
                const MANTISSA_BITS: u32 = $mantissa_bits;

                const TOTAL_BITS: u32 = 1 + EXPONENT_BITS + MANTISSA_BITS;

                // The spec always gives an exponent bias that follows this formula.
                const EXPONENT_BIAS: u32 = (1 << (EXPONENT_BITS - 1)) - 1;

                // 4 bits for each hexadecimal offset
                let mut exponent_offset: i32 = literal.decimal_offset * 4;

                // If there were all
                if literal.digits.is_empty() {
                    return ConversionResult::Precise(0.0);
                }

                // This code is a work of art.
                let mut was_truncated = false;
                let mut mantissa_result: $bits_type = 0;
                for (index, digit) in literal.digits.iter().enumerate() {
                    if index as u32 * 4 > MANTISSA_BITS {
                        was_truncated = true;
                        break;
                    }
                    let mut digit_value = *digit as $bits_type;
                    digit_value <<= TOTAL_BITS - (index as u32 + 1) * 4;
                    mantissa_result |= digit_value;
                }
                let leading_zeros = mantissa_result.leading_zeros();
                exponent_offset -= leading_zeros as i32 + 1;
                mantissa_result <<= leading_zeros + 1;
                mantissa_result >>= TOTAL_BITS - MANTISSA_BITS;

                let final_exponent = exponent_offset + literal.exponent;

                // Check for underflows
                if final_exponent < $min_exp - 1 {
                    // TODO: Implement subnormal numbers.
                    if literal.is_positive {
                        return ConversionResult::Imprecise(0.0);
                    } else {
                        return ConversionResult::Imprecise(-0.0);
                    };
                }

                // Check for overflows
                if final_exponent > $max_exp - 1 {
                    if literal.is_positive {
                        return ConversionResult::Imprecise($infinity);
                    } else {
                        return ConversionResult::Imprecise(-$infinity);
                    };
                }

                let exponent_result: $bits_type =
                    ((final_exponent + EXPONENT_BIAS as i32) as $bits_type) << MANTISSA_BITS;

                let sign_result: $bits_type =
                    (!literal.is_positive as $bits_type) << (MANTISSA_BITS + EXPONENT_BITS);

                let float_value = $from_bits(sign_result | exponent_result | mantissa_result);

                if was_truncated {
                    ConversionResult::Imprecise(float_value)
                } else {
                    ConversionResult::Precise(float_value)
                }

                // // This might be a bit faster.
                // let mut final_result = !literal.is_positive as $bits_type;
                // final_result <<= EXPONENT_BITS;
                // final_result |= (final_exponent + EXPONENT_BIAS as i32) as $bits_type;
                // final_result <<= MANTISSA_BITS;
                // final_result |= mantissa_result;
                // ConversionResult::Precise($from_bits(final_result))
            }
        }
    };
}

/// Trait to describe conversion to floating point formats.
pub trait FPFormat: ops::Neg<Output = Self> + Sized + Copy {
    /// Convert a literal to this format. This is a hack so that we can use
    /// a macro to implement conversions.
    fn from_literal(literal: FloatLiteral) -> ConversionResult<Self>;
}

impl_fpformat!(
    f32,
    u32,
    8,
    23,
    f32::from_bits,
    core::f32::INFINITY,
    core::f32::MAX_EXP,
    core::f32::MIN_EXP
);
impl_fpformat!(
    f64,
    u64,
    11,
    52,
    f64::from_bits,
    core::f64::INFINITY,
    core::f64::MAX_EXP,
    core::f64::MIN_EXP
);
