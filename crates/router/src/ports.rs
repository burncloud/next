use billing::model::TokenUsage;

pub trait Adapter {
    type Error;

    /// # Errors
    ///
    /// 响应体里没有用量，或用量读不出来时，返回 `Self::Error`，不能当成 0。
    fn read_usage(&self, body: &[u8]) -> Result<TokenUsage, Self::Error>;
}
