use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};

use billing::model::{ModelUsage, TokenUsage};
use kernel::model::ModelId;
use router::model::{Prepared, UpstreamReply};
use router::ports::{Adapter, ByteStream, Upstream, UsageMeter};

fn poll_once<T>(future: impl Future<Output = T>) -> Option<T> {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => Some(value),
        Poll::Pending => None,
    }
}

const USAGE: TokenUsage = TokenUsage {
    input_tokens: 3,
    cached_input_tokens: 5,
    output_tokens: 7,
};

#[derive(Debug, PartialEq, Eq)]
struct Meter {
    seen: Vec<u8>,
    failed: bool,
}

impl UsageMeter for Meter {
    type Error = ();

    fn feed(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.seen.extend_from_slice(bytes);
        if self.failed || bytes.contains(&b'!') {
            self.failed = true;
            return Err(());
        }
        Ok(())
    }

    fn ended(&self) -> bool {
        self.seen.ends_with(b"$")
    }

    fn finish(self) -> Result<TokenUsage, Self::Error> {
        if self.failed || !self.ended() {
            return Err(());
        }
        Ok(USAGE)
    }
}

struct Stub;

impl Adapter for Stub {
    type Error = ();
    type Meter = Meter;

    fn prepare(&self, body: &[u8]) -> Result<Prepared<Self::Meter>, Self::Error> {
        if body.is_empty() {
            return Err(());
        }
        Ok(Prepared {
            limit: ModelUsage {
                model: ModelId(String::from("m")),
                tokens: USAGE,
            },
            upstream_body: body.to_vec(),
            streaming: true,
            meter: Meter {
                seen: Vec::new(),
                failed: false,
            },
        })
    }
}

struct Chunks(Vec<Vec<u8>>);

impl ByteStream for Chunks {
    type Error = ();

    fn next_chunk(&mut self) -> impl Future<Output = Option<Result<Vec<u8>, Self::Error>>> + Send {
        let next = if self.0.is_empty() {
            None
        } else {
            Some(Ok(self.0.remove(0)))
        };
        core::future::ready(next)
    }
}

impl Upstream for Stub {
    type Error = ();
    type Body = Chunks;

    fn call(
        &self,
        body: Vec<u8>,
        streaming: bool,
    ) -> impl Future<Output = Result<UpstreamReply<Self::Body>, Self::Error>> + Send {
        let reply = if streaming {
            Ok(UpstreamReply {
                status: 200,
                body: Chunks(body.chunks(2).map(<[u8]>::to_vec).collect()),
            })
        } else {
            Err(())
        };
        core::future::ready(reply)
    }
}

fn run(body: &[u8]) -> (Vec<u8>, Result<TokenUsage, ()>) {
    let stub = Stub;
    let Ok(prepared) = stub.prepare(body) else {
        return (Vec::new(), Err(()));
    };
    let mut meter = prepared.meter;
    let Some(Ok(mut reply)) = poll_once(stub.call(prepared.upstream_body, prepared.streaming))
    else {
        return (Vec::new(), Err(()));
    };
    let mut forwarded = Vec::new();
    while let Some(Some(Ok(chunk))) = poll_once(reply.body.next_chunk()) {
        let _fed = meter.feed(&chunk);
        forwarded.extend_from_slice(&chunk);
    }
    (forwarded, meter.finish())
}

#[test]
fn prepare_returns_limit_body_and_meter() {
    let stub = Stub;
    assert_eq!(stub.prepare(b""), Err(()));
    let prepared = stub.prepare(b"{}");
    assert_eq!(
        prepared.map(|p| (p.limit, p.upstream_body, p.streaming)),
        Ok((
            ModelUsage {
                model: ModelId(String::from("m")),
                tokens: USAGE,
            },
            b"{}".to_vec(),
            true,
        ))
    );
}

#[test]
fn upstream_reply_carries_status_and_stream() {
    let stub = Stub;
    assert!(matches!(
        poll_once(stub.call(b"x".to_vec(), false)),
        Some(Err(()))
    ));
    let reply = poll_once(stub.call(b"abc".to_vec(), true));
    assert!(matches!(reply, Some(Ok(UpstreamReply { status: 200, .. }))));
}

#[test]
fn meter_reads_usage_across_chunks_after_normal_end() {
    assert_eq!(run(b"data$"), (b"data$".to_vec(), Ok(USAGE)));
}

#[test]
fn meter_error_does_not_stop_forwarding() {
    assert_eq!(run(b"d!ta$"), (b"d!ta$".to_vec(), Err(())));
}

#[test]
fn meter_without_end_marker_fails() {
    let mut meter = Meter {
        seen: Vec::new(),
        failed: false,
    };
    assert_eq!(meter.feed(b"data"), Ok(()));
    assert!(!meter.ended());
    assert_eq!(meter.finish(), Err(()));
}
