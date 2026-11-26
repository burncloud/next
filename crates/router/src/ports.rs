use billing::model::TokenUsage;

pub trait Adapter {
    type Error;

    /// # Errors
    ///
    /// 响应体里没有用量，或用量读不出来时，返回 `Self::Error`，不能当成 0。
    fn read_usage(&self, body: &[u8]) -> Result<TokenUsage, Self::Error>;

    /// 按请求体估算用量上限（输入 token 加 `max_tokens`），用来定冻结额。请求体没带 `max_tokens` 时按该模型的最大输出上限估算，不拒绝。
    ///
    /// # Errors
    ///
    /// 请求体读不出来时返回 `Self::Error`。
    fn estimate_usage(&self, body: &[u8]) -> Result<TokenUsage, Self::Error>;
}

pub trait Upstream {
    type Error;

    /// 把请求体发给上游，返回响应体。
    ///
    /// # Errors
    ///
    /// 上游调用失败时返回 `Self::Error`。
    fn call(&self, body: &[u8]) -> Result<Vec<u8>, Self::Error>;
}
