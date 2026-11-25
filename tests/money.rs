use next::money::{Currency, CurrencyError, Money, MoneyError};
use std::str::FromStr;

fn usd(text: &str) -> Result<Money, MoneyError> {
    Money::parse_decimal(Currency::Usd, text)
}

#[test]
fn parses_major_units_into_nanos() {
    let money = usd("2.5").unwrap();
    assert_eq!(money.currency(), Currency::Usd);
    assert_eq!(money.minor_units(), 2_500_000_000);
    assert_eq!(money.to_decimal_string(), "2.5");
}

#[test]
fn rounds_half_away_from_zero_at_the_nano_boundary() {
    assert_eq!(usd("0.000000001").unwrap().minor_units(), 1);
    assert_eq!(usd("0.0000000014").unwrap().minor_units(), 1);
    assert_eq!(usd("0.0000000015").unwrap().minor_units(), 2);
    assert_eq!(usd("1e-9").unwrap().minor_units(), 1);
    assert_eq!(usd("1.5e1").unwrap().minor_units(), 15_000_000_000);
}

#[test]
fn accepts_zero_and_signed_zero() {
    assert_eq!(usd("0").unwrap().minor_units(), 0);
    assert_eq!(usd("-0").unwrap().minor_units(), 0);
    assert_eq!(usd("-0.0000000004").unwrap().minor_units(), 0);
    assert_eq!(usd("+2").unwrap().minor_units(), 2_000_000_000);
}

#[test]
fn rejects_negative_prices() {
    assert_eq!(usd("-0.01"), Err(MoneyError::Negative));
    assert_eq!(usd("-0.0000000005"), Err(MoneyError::Negative));
    assert_eq!(
        Money::from_minor_units(Currency::Usd, -1),
        Err(MoneyError::Negative)
    );
}

#[test]
fn rejects_nan_and_infinity() {
    assert_eq!(usd("NaN"), Err(MoneyError::NotANumber));
    assert_eq!(usd("nan"), Err(MoneyError::NotANumber));
    assert_eq!(usd("+NaN"), Err(MoneyError::NotANumber));
    assert_eq!(usd("-NaN"), Err(MoneyError::NotANumber));
    assert_eq!(usd("Infinity"), Err(MoneyError::Infinite));
    assert_eq!(usd("+Infinity"), Err(MoneyError::Infinite));
    assert_eq!(usd("-Infinity"), Err(MoneyError::Infinite));
    assert_eq!(usd("inf"), Err(MoneyError::Infinite));
}

#[test]
fn rejects_invalid_decimals_and_overflow() {
    assert_eq!(usd(""), Err(MoneyError::InvalidDecimal));
    assert_eq!(usd("abc"), Err(MoneyError::InvalidDecimal));
    assert_eq!(usd("1.2.3"), Err(MoneyError::InvalidDecimal));
    assert_eq!(usd("1e"), Err(MoneyError::InvalidDecimal));
    assert_eq!(usd("1e20"), Err(MoneyError::Overflow));
}

#[test]
fn checked_arithmetic_reports_overflow_and_currency_mismatch() {
    let left = Money::from_minor_units(Currency::Usd, 2).unwrap();
    let right = Money::from_minor_units(Currency::Usd, 3).unwrap();
    assert_eq!(left.checked_add(right).unwrap().minor_units(), 5);
    assert_eq!(left.checked_sub(right), Err(MoneyError::Negative));
    assert_eq!(right.checked_sub(left).unwrap().minor_units(), 1);
    assert_eq!(left.checked_mul(4).unwrap().minor_units(), 8);

    let cny = Money::from_minor_units(Currency::Cny, 1).unwrap();
    assert_eq!(left.checked_add(cny), Err(MoneyError::CurrencyMismatch));
    assert_eq!(left.checked_sub(cny), Err(MoneyError::CurrencyMismatch));

    let max = Money::from_minor_units(Currency::Usd, i64::MAX).unwrap();
    let one = Money::from_minor_units(Currency::Usd, 1).unwrap();
    assert_eq!(max.checked_add(one), Err(MoneyError::Overflow));
    assert_eq!(max.checked_mul(2), Err(MoneyError::Overflow));
}

#[test]
fn currency_accepts_only_uppercase_iso_codes_and_symbols() {
    assert_eq!(Currency::from_str("USD").unwrap(), Currency::Usd);
    assert_eq!(Currency::parse("CNY").unwrap().code(), "CNY");
    assert_eq!(Currency::from_str("usd"), Err(CurrencyError::InvalidCode));
    assert_eq!(Currency::from_str("GBP"), Err(CurrencyError::InvalidCode));
    assert_eq!(Currency::from_str("US"), Err(CurrencyError::InvalidCode));
    assert_eq!(Currency::Usd.symbol(), "$");
    assert_eq!(Currency::Cny.symbol(), "\u{00A5}");
    assert_eq!(Currency::Eur.symbol(), "\u{20AC}");
    assert_eq!(Currency::Usd.to_string(), "USD");

    assert_eq!(serde_json::to_string(&Currency::Eur).unwrap(), "\"EUR\"");
    assert!(serde_json::from_str::<Currency>("\"usd\"").is_err());
    assert_eq!(
        serde_json::from_str::<Currency>("\"USD\"").unwrap(),
        Currency::Usd
    );
}
