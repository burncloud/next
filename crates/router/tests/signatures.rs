use billing::model::{ModelUsage, TokenUsage};
use router::ports::{Adapter, Upstream};

struct Stub;

impl Adapter for Stub {
    type Error = ();

    fn read_usage(&self, body: &[u8]) -> Result<TokenUsage, Self::Error> {
        if body.is_empty() {
            return Err(());
        }
        Ok(TokenUsage {
            input_tokens: 3,
            cached_input_tokens: 5,
            output_tokens: 7,
        })
    }

    fn estimate_usage(&self, _body: &[u8]) -> Result<ModelUsage, Self::Error> {
        Err(())
    }
}

impl Upstream for Stub {
    type Error = ();

    fn call(&self, body: &[u8]) -> Result<Vec<u8>, Self::Error> {
        Ok(body.to_vec())
    }
}

#[test]
fn adapter_and_upstream_signatures_are_usable() {
    let stub = Stub;
    let usage = stub.read_usage(b"{}");
    assert_eq!(
        usage,
        Ok(TokenUsage {
            input_tokens: 3,
            cached_input_tokens: 5,
            output_tokens: 7,
        })
    );
    assert_eq!(stub.read_usage(b""), Err(()));
    assert_eq!(stub.estimate_usage(b"{}"), Err(()));
    assert_eq!(stub.call(b"x"), Ok(b"x".to_vec()));
}
