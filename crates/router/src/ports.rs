use billing::model::{ModelUsage, TokenUsage};

pub trait Adapter {
    type Error;

    /// # Errors
    ///
    /// 响应体里没有用量、用量读不出来、缓存命中数大于上游给的输入总数，或出现不支持的计费项时，返回 `Self::Error`，不能当成 0。
    fn read_usage(&self, body: &[u8]) -> Result<TokenUsage, Self::Error>;

    /// 按请求体估算用量上限，用来定冻结额。`cached_input_tokens` 为 0。`output_tokens` 取请求的最大输出；请求没带时，取实现给该模型配置的最大输出上限。
    ///
    /// # Errors
    ///
    /// 请求体读不出来、没配该模型的最大输出上限，或请求了不支持的计费档位时，返回 `Self::Error`，不能按 0、默认值或其他模型估算。
    fn estimate_usage(&self, body: &[u8]) -> Result<ModelUsage, Self::Error>;
}

pub trait Upstream {
    type Error;

    /// # Errors
    ///
    /// 上游调用失败时返回 `Self::Error`。
    fn call(&self, body: &[u8]) -> Result<Vec<u8>, Self::Error>;
}
