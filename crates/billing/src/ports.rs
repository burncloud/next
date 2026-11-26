use kernel::model::{AccountId, Currency, Money};

pub trait Pricing {
    type Usage;
    type Error;

    /// # Errors
    ///
    /// 用量无法计价时返回 `Self::Error`。
    fn price(&self, usage: &Self::Usage) -> Result<Money, Self::Error>;
}

pub trait Wallet {
    type Error;

    /// # Errors
    ///
    /// 读不到该账号该币种的余额时返回 `Self::Error`。
    fn balance(&self, account: &AccountId, currency: Currency) -> Result<Money, Self::Error>;

    /// # Errors
    ///
    /// 余额不足或扣费失败时返回 `Self::Error`。
    fn charge(&self, account: &AccountId, amount: Money) -> Result<(), Self::Error>;
}
