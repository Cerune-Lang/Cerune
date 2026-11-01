/// Formats a value using Cerune output semantics without performing I/O.
pub fn f32(value: f32) -> String {
    significant(f64::from(value), 9)
}

/// Formats a value using Cerune output semantics without performing I/O.
pub fn f64(value: f64) -> String {
    significant(value, 17)
}

// 生成コードの%.9g / %.17gと同じく、有効数字で丸めた後の指数で表記を選びます。
// log10や乗除算で桁を調整せず、非正規化数の丸めも標準の書式処理に任せます。
fn significant(value: f64, digits: usize) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let scientific = format!("{value:.precision$e}", precision = digits - 1);
    let (mantissa, exponent) = scientific.split_once('e').expect("scientific exponent");
    let exponent: i32 = exponent.parse().expect("decimal exponent");
    if exponent < -4 || exponent >= digits as i32 {
        format!("{}e{exponent:+03}", trim_decimal(mantissa))
    } else {
        // 再整形するのは元の値です。丸め済みの文字列を数値へ戻して二重に丸めません。
        let precision = (digits as i32 - exponent - 1) as usize;
        trim_decimal(&format!("{value:.precision$}")).to_owned()
    }
}

fn trim_decimal(text: &str) -> &str {
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn general_format_selects_notation_after_rounding() {
        for (value, digits, expected) in [
            (0.0, 9, "0"),
            (-0.0, 17, "-0"),
            (1.5, 17, "1.5"),
            (0.0001, 9, "0.0001"),
            (0.00001, 9, "1e-05"),
            (999999999.0, 9, "999999999"),
            (1e9, 9, "1e+09"),
            (999999999.9, 9, "1e+09"),
            (0.00009999999999, 9, "0.0001"),
            (1.25, 2, "1.2"),
            (1.75, 2, "1.8"),
            (-1.25, 2, "-1.2"),
        ] {
            assert_eq!(significant(value, digits), expected, "{value}, {digits}");
        }
    }

    #[test]
    fn extreme_values_keep_their_sign_and_magnitude() {
        assert_eq!(f32(f32::from_bits(1)), "1.40129846e-45");
        assert_eq!(f32(f32::MAX), "3.40282347e+38");
        assert_eq!(f64(f64::from_bits(1)), "4.9406564584124654e-324");
        assert_eq!(f64(f64::MAX), "1.7976931348623157e+308");
        assert_eq!(f64(-1e-20), "-9.9999999999999995e-21");
        assert_eq!(f64(f64::INFINITY), "inf");
        assert_eq!(f64(f64::NEG_INFINITY), "-inf");
        assert_eq!(f64(f64::NAN), "NaN");
    }

    #[test]
    fn f32_output_round_trips_across_exponents_and_signs() {
        for exponent in 0..255u32 {
            for fraction in [0, 1, 0x155555, 0x400000, 0x7ffffe, 0x7fffff] {
                for sign in [0, 1u32 << 31] {
                    let bits = sign | (exponent << 23) | fraction;
                    let text = f32(f32::from_bits(bits));
                    assert_eq!(text.parse::<f32>().unwrap().to_bits(), bits, "{text}");
                }
            }
        }
    }

    #[test]
    fn f64_output_round_trips_across_exponents_and_signs() {
        for exponent in 0..2047u64 {
            for fraction in [
                0,
                1,
                0x5555555555555,
                0x8000000000000,
                0xffffffffffffe,
                0xfffffffffffff,
            ] {
                for sign in [0, 1u64 << 63] {
                    let bits = sign | (exponent << 52) | fraction;
                    let text = f64(f64::from_bits(bits));
                    assert_eq!(text.parse::<f64>().unwrap().to_bits(), bits, "{text}");
                }
            }
        }
    }
}
