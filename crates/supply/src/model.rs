use kernel::model::ModelId;

/// 渠道标识。
///
/// 只有名字。不含上游地址、密钥或其它下游凭证。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChannelId(pub String);

/// 一个模型对应一个渠道。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelRoute {
    /// 与查询用的 `ModelId` 逐字相同。
    pub model: ModelId,
    /// 该模型所在的渠道。
    pub channel: ChannelId,
}
