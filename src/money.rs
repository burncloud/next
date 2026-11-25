use std::error::Error;
use std::fmt;

pub const MINOR_UNITS_PER_MAJOR: i64 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[must_use]
pub enum Currency {
    Usd,
    Cny,
    Eur,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub struct Money {
    currency: Currency,
    minor_units: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub enum MoneyError {
    Negative,
    Overflow,
    CurrencyMismatch,
    InvalidCurrency,
}

pub trait Pricing {
    type Usage;
    type Error;

    /// Prices one `usage` value as [`Money`].
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] when the implementor rejects `usage`.
    /// This method does not return [`MoneyError`]; the implementor selects [`Self::Error`].
    fn price(&self, usage: &Self::Usage) -> Result<Money, Self::Error>;
}

impl Currency {
    /// Parses an uppercase ISO 4217 code.
    ///
    /// # Errors
    ///
    /// Returns [`MoneyError::InvalidCurrency`] when `text` is not `USD`, `CNY`, or `EUR`.
    pub const fn parse(text: &str) -> Result<Self, MoneyError> {
        match text.as_bytes() {
            b"USD" => Ok(Self::Usd),
            b"CNY" => Ok(Self::Cny),
            b"EUR" => Ok(Self::Eur),
            _ => Err(MoneyError::InvalidCurrency),
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

impl Money {
    /// Builds a non-negative amount in minor units.
    ///
    /// # Errors
    ///
    /// Returns [`MoneyError::Negative`] when `minor_units` is below zero.
    pub const fn new(currency: Currency, minor_units: i64) -> Result<Self, MoneyError> {
        if minor_units < 0 {
            return Err(MoneyError::Negative);
        }
        Ok(Self {
            currency,
            minor_units,
        })
    }

    pub const fn currency(self) -> Currency {
        self.currency
    }

    #[must_use]
    pub const fn minor_units(self) -> i64 {
        self.minor_units
    }

    /// Adds two amounts of the same currency.
    ///
    /// # Errors
    ///
    /// Returns [`MoneyError::CurrencyMismatch`] when the currencies differ.
    /// Returns [`MoneyError::Overflow`] when the sum does not fit in `i64`.
    pub const fn checked_add(self, rhs: Self) -> Result<Self, MoneyError> {
        if !same_currency(self.currency, rhs.currency) {
            return Err(MoneyError::CurrencyMismatch);
        }
        match self.minor_units.checked_add(rhs.minor_units) {
            Some(minor_units) => Ok(Self {
                currency: self.currency,
                minor_units,
            }),
            None => Err(MoneyError::Overflow),
        }
    }

    /// Subtracts one amount from another of the same currency.
    ///
    /// # Errors
    ///
    /// Returns [`MoneyError::CurrencyMismatch`] when the currencies differ.
    /// Returns [`MoneyError::Negative`] when the difference is below zero.
    /// Returns [`MoneyError::Overflow`] when the difference does not fit in `i64`.
    pub const fn checked_sub(self, rhs: Self) -> Result<Self, MoneyError> {
        if !same_currency(self.currency, rhs.currency) {
            return Err(MoneyError::CurrencyMismatch);
        }
        match self.minor_units.checked_sub(rhs.minor_units) {
            Some(minor_units) if minor_units < 0 => Err(MoneyError::Negative),
            Some(minor_units) => Ok(Self {
                currency: self.currency,
                minor_units,
            }),
            None => Err(MoneyError::Overflow),
        }
    }
}

const fn same_currency(left: Currency, right: Currency) -> bool {
    matches!(
        (left, right),
        (Currency::Usd, Currency::Usd)
            | (Currency::Cny, Currency::Cny)
            | (Currency::Eur, Currency::Eur)
    )
}

impl fmt::Display for MoneyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Negative => "negative amount",
            Self::Overflow => "amount overflow",
            Self::CurrencyMismatch => "currency mismatch",
            Self::InvalidCurrency => "invalid currency code",
        };
        formatter.write_str(text)
    }
}

impl Error for MoneyError {}
