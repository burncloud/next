use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

pub const MINOR_UNITS_PER_MAJOR: i64 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[must_use]
pub enum Currency {
    Usd,
    Cny,
    Eur,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[must_use]
pub enum CurrencyError {
    #[error("invalid currency code")]
    InvalidCode,
}

impl Currency {
    pub const fn parse(text: &str) -> Result<Self, CurrencyError> {
        match text.as_bytes() {
            b"USD" => Ok(Self::Usd),
            b"CNY" => Ok(Self::Cny),
            b"EUR" => Ok(Self::Eur),
            _ => Err(CurrencyError::InvalidCode),
        }
    }

    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Usd => "USD",
            Self::Cny => "CNY",
            Self::Eur => "EUR",
        }
    }

    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Usd => "$",
            Self::Cny => "\u{00A5}",
            Self::Eur => "\u{20AC}",
        }
    }
}

impl FromStr for Currency {
    type Err = CurrencyError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub struct Money {
    currency: Currency,
    minor: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[must_use]
pub enum MoneyError {
    #[error("negative amount")]
    Negative,
    #[error("amount is NaN")]
    NotANumber,
    #[error("amount is infinite")]
    Infinite,
    #[error("amount overflow")]
    Overflow,
    #[error("currency mismatch")]
    CurrencyMismatch,
    #[error("invalid decimal")]
    InvalidDecimal,
}

impl Money {
    pub fn parse_decimal(currency: Currency, text: &str) -> Result<Self, MoneyError> {
        Ok(Self {
            currency,
            minor: parse_decimal_to_minor(text)?,
        })
    }

    pub const fn from_minor_units(currency: Currency, minor: i64) -> Result<Self, MoneyError> {
        if minor < 0 {
            return Err(MoneyError::Negative);
        }
        Ok(Self { currency, minor })
    }

    pub const fn currency(self) -> Currency {
        self.currency
    }

    #[must_use]
    pub const fn minor_units(self) -> i64 {
        self.minor
    }

    #[must_use]
    pub fn to_decimal_string(self) -> String {
        let whole = self.minor / MINOR_UNITS_PER_MAJOR;
        let fraction = self.minor % MINOR_UNITS_PER_MAJOR;
        if fraction == 0 {
            return whole.to_string();
        }
        let mut digits = format!("{fraction:09}");
        while digits.ends_with('0') {
            digits.pop();
        }
        format!("{whole}.{digits}")
    }

    pub const fn checked_add(self, rhs: Self) -> Result<Self, MoneyError> {
        if !currency_eq(self.currency, rhs.currency) {
            return Err(MoneyError::CurrencyMismatch);
        }
        match self.minor.checked_add(rhs.minor) {
            Some(minor) => Ok(Self {
                currency: self.currency,
                minor,
            }),
            None => Err(MoneyError::Overflow),
        }
    }

    pub const fn checked_sub(self, rhs: Self) -> Result<Self, MoneyError> {
        if !currency_eq(self.currency, rhs.currency) {
            return Err(MoneyError::CurrencyMismatch);
        }
        match self.minor.checked_sub(rhs.minor) {
            Some(minor) if minor < 0 => Err(MoneyError::Negative),
            Some(minor) => Ok(Self {
                currency: self.currency,
                minor,
            }),
            None => Err(MoneyError::Overflow),
        }
    }

    pub fn checked_mul(self, factor: u64) -> Result<Self, MoneyError> {
        let Some(product) = i128::from(self.minor).checked_mul(i128::from(factor)) else {
            return Err(MoneyError::Overflow);
        };
        let minor = i64::try_from(product).map_err(|_| MoneyError::Overflow)?;
        Ok(Self {
            currency: self.currency,
            minor,
        })
    }
}

const fn currency_eq(left: Currency, right: Currency) -> bool {
    matches!(
        (left, right),
        (Currency::Usd, Currency::Usd)
            | (Currency::Cny, Currency::Cny)
            | (Currency::Eur, Currency::Eur)
    )
}

fn parse_decimal_to_minor(text: &str) -> Result<i64, MoneyError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(MoneyError::InvalidDecimal);
    }
    let (negative, rest) = match text.strip_prefix('+') {
        Some(rest) => (false, rest),
        None => match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text),
        },
    };
    if rest.is_empty() {
        return Err(MoneyError::InvalidDecimal);
    }
    if is_nan_token(rest) {
        return Err(MoneyError::NotANumber);
    }
    if is_infinite_token(rest) {
        return Err(MoneyError::Infinite);
    }
    let (coefficient, exponent) = split_exponent(rest)?;
    let (whole, fraction) = split_point(coefficient)?;
    let digits = significand(whole, fraction);
    let value = parse_digits(&digits)?;
    let fraction_len = i32::try_from(fraction.len()).map_err(|_| MoneyError::Overflow)?;
    let shift = exponent
        .checked_sub(fraction_len)
        .and_then(|item| item.checked_add(9))
        .ok_or(MoneyError::Overflow)?;
    let scaled = scale(value, shift)?;
    if negative && scaled != 0 {
        return Err(MoneyError::Negative);
    }
    i64::try_from(scaled).map_err(|_| MoneyError::Overflow)
}

fn is_nan_token(text: &str) -> bool {
    text.eq_ignore_ascii_case("nan")
}

fn is_infinite_token(text: &str) -> bool {
    text.eq_ignore_ascii_case("inf") || text.eq_ignore_ascii_case("infinity")
}

fn split_exponent(text: &str) -> Result<(&str, i32), MoneyError> {
    let Some(index) = text.bytes().position(|byte| byte == b'e' || byte == b'E') else {
        return Ok((text, 0));
    };
    let (coefficient, exponent) = text.split_at(index);
    if coefficient.is_empty() || exponent.len() == 1 {
        return Err(MoneyError::InvalidDecimal);
    }
    Ok((coefficient, parse_exponent(&exponent[1..])?))
}

fn parse_exponent(text: &str) -> Result<i32, MoneyError> {
    let (sign, digits) = match text.strip_prefix('+') {
        Some(rest) => (1_i32, rest),
        None => match text.strip_prefix('-') {
            Some(rest) => (-1, rest),
            None => (1, text),
        },
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(MoneyError::InvalidDecimal);
    }
    if digits.len() > 9 {
        return Err(MoneyError::Overflow);
    }
    let magnitude: i32 = digits.parse().map_err(|_| MoneyError::Overflow)?;
    magnitude.checked_mul(sign).ok_or(MoneyError::Overflow)
}

fn split_point(text: &str) -> Result<(&str, &str), MoneyError> {
    if let Some((whole, rest)) = text.split_once('.') {
        if rest.contains('.') {
            return Err(MoneyError::InvalidDecimal);
        }
        digits_only(whole, false)?;
        digits_only(rest, true)?;
        return Ok((whole, rest));
    }
    digits_only(text, false)?;
    Ok((text, ""))
}

fn digits_only(text: &str, allow_empty: bool) -> Result<(), MoneyError> {
    if text.is_empty() {
        return if allow_empty {
            Ok(())
        } else {
            Err(MoneyError::InvalidDecimal)
        };
    }
    if text.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(())
    } else {
        Err(MoneyError::InvalidDecimal)
    }
}

fn significand(whole: &str, fraction: &str) -> String {
    let trimmed = whole.trim_start_matches('0');
    let mut digits = String::with_capacity(trimmed.len().saturating_add(fraction.len()));
    digits.push_str(trimmed);
    digits.push_str(fraction);
    digits
}

fn parse_digits(digits: &str) -> Result<i128, MoneyError> {
    if digits.is_empty() {
        return Ok(0);
    }
    digits.parse::<i128>().map_err(|_| MoneyError::Overflow)
}

fn scale(value: i128, shift: i32) -> Result<i128, MoneyError> {
    if value == 0 {
        return Ok(0);
    }
    if shift > 40 {
        return Err(MoneyError::Overflow);
    }
    if shift >= 0 {
        let mut acc = value;
        let times = usize::try_from(shift).map_err(|_| MoneyError::Overflow)?;
        for _ in 0..times {
            acc = acc.checked_mul(10).ok_or(MoneyError::Overflow)?;
        }
        return Ok(acc);
    }
    let negative_shift = shift.checked_neg().ok_or(MoneyError::Overflow)?;
    let times = usize::try_from(negative_shift).map_err(|_| MoneyError::Overflow)?;
    if times > 38 {
        return Ok(0);
    }
    let mut denom: i128 = 1;
    for _ in 0..times {
        denom = denom.checked_mul(10).ok_or(MoneyError::Overflow)?;
    }
    let quotient = value / denom;
    let remainder = value % denom;
    let doubled = remainder.checked_mul(2).ok_or(MoneyError::Overflow)?;
    if doubled >= denom {
        quotient.checked_add(1).ok_or(MoneyError::Overflow)
    } else {
        Ok(quotient)
    }
}
