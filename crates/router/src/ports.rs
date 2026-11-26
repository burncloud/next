use billing::model::TokenUsage;

pub trait Adapter {
    type Error;

    /// # Errors
    ///
    /// 响应体里的用量读不出来时返回 `Self::Error`。缺字段不是错误，读成 0。
    fn read_usage(&self, body: &[u8]) -> Result<TokenUsage, Self::Error>;
}
