//! Fixed character-budget prices for dense quote views. Never changes quote data.

const DISPLAY_SLOTS: usize = 8;
const SUBSCRIPT_DIGITS: [char; 10] = ['₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉'];

pub(crate) fn full_price(price: f64) -> String {
    if !price.is_finite() {
        return "--".into();
    }
    if price == 0.0 {
        return "0".into();
    }
    let full = price.to_string();
    if full.len() > 40 {
        // Shortest round-trippable scientific value for extreme magnitudes.
        format!("{price:e}")
    } else {
        full
    }
}

pub(crate) fn compact_price(price: f64) -> String {
    if !price.is_finite() || price == 0.0 {
        return full_price(price);
    }
    let magnitude = price.abs();
    let sign = if price.is_sign_negative() { "-" } else { "" };
    let plain = magnitude.to_string();
    let (integer, fraction) = plain.split_once('.').unwrap_or((&plain, ""));
    let zeros = fraction.bytes().take_while(|digit| *digit == b'0').count();
    let precision = if integer == "0" && zeros >= 4 {
        // The decimal point, literal zero and every subscript digit consume slots.
        zeros + DISPLAY_SLOTS.saturating_sub(sign.len() + 3 + zeros.to_string().len())
    } else {
        fraction
            .len()
            .min(DISPLAY_SLOTS.saturating_sub(sign.len() + integer.len() + 1))
    };
    // Reformat from the original value after carry (e.g. 0.000099999 -> 0.0001).
    for decimals in (0..=precision).rev() {
        let rounded = format!("{magnitude:.decimals$}");
        let formatted = compact_decimal(&rounded);
        if sign.len() + formatted.chars().count() <= DISPLAY_SLOTS {
            return format!("{sign}{formatted}");
        }
    }
    // Integers longer than the budget still retain their order of magnitude.
    for decimals in (0..DISPLAY_SLOTS).rev() {
        let scientific = format!("{price:.decimals$e}");
        if scientific.len() <= DISPLAY_SLOTS {
            return scientific;
        }
    }
    format!("{price:.0e}")
}

fn compact_decimal(value: &str) -> String {
    if let Some(fraction) = value.strip_prefix("0.") {
        let zeros = fraction.bytes().take_while(|digit| *digit == b'0').count();
        if zeros >= 4 && zeros < fraction.len() {
            let count: String = zeros
                .to_string()
                .bytes()
                .map(|digit| SUBSCRIPT_DIGITS[usize::from(digit - b'0')])
                .collect();
            return format!("0.0{count}{}", &fraction[zeros..]);
        }
    }
    if value.contains('.') {
        value.trim_end_matches('0').trim_end_matches('.').into()
    } else {
        value.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_slots_cover_major_assets_and_pepe() {
        for (price, expected) in [
            (68000.12, "68000.12"),
            (106800.12, "106800.1"),
            (3420.56, "3420.56"),
            (152.3478, "152.3478"),
            (602.45, "602.45"),
            (0.000004110, "0.0₅4110"),
            (0.00000001234, "0.0₇1234"),
            (0.00000000000123456, "0.0₁₁123"),
            (-0.000004110, "-0.0₅411"),
            (0.0, "0"),
            (-0.0, "0"),
        ] {
            assert_eq!(compact_price(price), expected, "{price}");
            assert!(expected.chars().count() <= DISPLAY_SLOTS);
        }
    }

    #[test]
    fn rounding_recomputes_zero_counts_and_integer_carries() {
        for (price, expected) in [
            (0.0000099999, "0.0₄1000"),
            (0.0000999999, "0.0001"),
            (99999.999, "100000"),
            (0.99999999, "1"),
            (123.456789, "123.4568"),
            (99999999.9, "1.0000e8"),
        ] {
            assert_eq!(compact_price(price), expected, "{price}");
        }
    }

    #[test]
    fn finite_extremes_fit_and_full_values_round_trip() {
        for price in [
            f64::MAX,
            -f64::MAX,
            f64::MIN_POSITIVE,
            f64::from_bits(1),
            -f64::from_bits(1),
            987654321.0,
            106800.12345678,
            0.000004110123456,
        ] {
            let compact = compact_price(price);
            assert!(
                compact.chars().count() <= DISPLAY_SLOTS,
                "{price}: {compact}"
            );
            assert_ne!(compact, "0");
            assert_eq!(full_price(price).parse::<f64>().unwrap(), price);
        }
        assert_eq!(full_price(106800.12345678), "106800.12345678");
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(compact_price(invalid), "--");
            assert_eq!(full_price(invalid), "--");
        }
    }
}
