use kernel::model::Money;

pub trait Pricing {
    type Usage;
    type Error;

    /// # Errors
    ///
    /// 用量无法计价时返回 `Self::Error`。
    fn price(&self, usage: &Self::Usage) -> Result<Money, Self::Error>;
}
