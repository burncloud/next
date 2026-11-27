use billing::model::ModelUsage;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Prepared<M> {
    /// 冻结用的用量上限。
    pub limit: ModelUsage,
    pub upstream_body: Vec<u8>,
    pub streaming: bool,
    pub meter: M,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UpstreamReply<B> {
    pub status: u16,
    pub body: B,
}
