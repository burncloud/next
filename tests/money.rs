use next::{Currency, Money, MoneyError, Pricing, MINOR_UNITS_PER_MAJOR};

fn usd(minor_units: i64) -> Money {
    Money::new(Currency::Usd, minor_units).unwrap()
}

#[test]
fn scale_constant_is_one_billion_nanos() {
    assert_eq!(MINOR_UNITS_PER_MAJOR, 10_i64.pow(9));
}

#[test]
fn new_accepts_zero_and_rejects_negative() {
    let zero = Money::new(Currency::Usd, 0).unwrap();
    assert_eq!(zero.currency(), Currency::Usd);
    assert_eq!(zero.minor_units(), 0);
    assert_eq!(Money::new(Currency::Cny, -1), Err(MoneyError::Negative));
}

#[test]
fn checked_add_and_sub_succeed_for_the_same_currency() {
    let left = usd(2);
    let right = usd(3);
    assert_eq!(left.checked_add(right).unwrap().minor_units(), 5);
    assert_eq!(right.checked_sub(left).unwrap().minor_units(), 1);
    assert_eq!(left.checked_sub(left).unwrap().minor_units(), 0);
}

#[test]
fn checked_arithmetic_reports_overflow_mismatch_and_negative() {
    let max = usd(i64::MAX);
    let one = usd(1);
    assert_eq!(max.checked_add(one), Err(MoneyError::Overflow));
    assert_eq!(one.checked_sub(usd(2)), Err(MoneyError::Negative));

    let cny = Money::new(Currency::Cny, MINOR_UNITS_PER_MAJOR).unwrap();
    assert_eq!(one.checked_add(cny), Err(MoneyError::CurrencyMismatch));
    assert_eq!(one.checked_sub(cny), Err(MoneyError::CurrencyMismatch));
}

#[test]
fn currency_parse_accepts_only_uppercase_codes_and_symbols() {
    assert_eq!(Currency::parse("USD").unwrap(), Currency::Usd);
    assert_eq!(Currency::parse("CNY").unwrap().code(), "CNY");
    assert_eq!(Currency::parse("EUR").unwrap().symbol(), "\u{20AC}");
    assert_eq!(Currency::Usd.symbol(), "$");
    assert_eq!(Currency::Cny.symbol(), "\u{00A5}");
    assert_eq!(Currency::parse("usd"), Err(MoneyError::InvalidCurrency));
    assert_eq!(Currency::parse("GBP"), Err(MoneyError::InvalidCurrency));
    assert_eq!(Currency::parse("US"), Err(MoneyError::InvalidCurrency));
    assert_eq!(
        MoneyError::InvalidCurrency.to_string(),
        "invalid currency code"
    );
}

struct SamplePricer;

#[derive(Debug, PartialEq, Eq)]
struct SampleError;

impl Pricing for SamplePricer {
    type Usage = u64;
    type Error = SampleError;

    fn price(&self, usage: &Self::Usage) -> Result<Money, Self::Error> {
        if *usage == 0 {
            return Err(SampleError);
        }
        Money::new(Currency::Usd, MINOR_UNITS_PER_MAJOR).map_err(|_| SampleError)
    }
}

#[test]
fn pricing_trait_can_return_ok_and_err() {
    let pricer = SamplePricer;
    assert_eq!(
        pricer.price(&1).unwrap().minor_units(),
        MINOR_UNITS_PER_MAJOR
    );
    assert_eq!(pricer.price(&0), Err(SampleError));
}
