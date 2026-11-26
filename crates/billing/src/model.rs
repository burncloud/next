#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}
