use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};

use kernel::model::{AccountId, Currency, KeyId, Money, RequestId, UnixMillis};
use router::ports::ByteStream;
use runtime::errors::Rejection;
use runtime::events::{ActualCost, SettlementCapped, UsageUnreadable};
use runtime::model::Reply;
use runtime::ports::Gateway;

fn poll_once<T>(future: impl Future<Output = T>) -> Option<T> {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => Some(value),
        Poll::Pending => None,
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Failure {
    Rejected(Rejection),
}

impl From<Rejection> for Failure {
    fn from(rejection: Rejection) -> Self {
        Self::Rejected(rejection)
    }
}

struct Once(Option<Vec<u8>>);

impl ByteStream for Once {
    type Error = ();

    fn next_chunk(&mut self) -> impl Future<Output = Option<Result<Vec<u8>, Self::Error>>> + Send {
        core::future::ready(self.0.take().map(Ok))
    }
}

struct Stub;

impl Gateway for Stub {
    type Error = Failure;
    type Body = Once;

    fn handle(
        &self,
        key: &str,
        idempotency_key: Option<&str>,
        body: &[u8],
    ) -> impl Future<Output = Result<Reply<Self::Body>, Self::Error>> + Send {
        let reply = match (key, idempotency_key) {
            ("", _) => Err(Rejection::Unauthenticated.into()),
            (_, Some(_)) => Err(Rejection::DuplicateRequest.into()),
            _ => Ok(Reply {
                status: 200,
                streaming: true,
                body: Once(Some(body.to_vec())),
            }),
        };
        core::future::ready(reply)
    }
}

#[test]
fn gateway_is_async_and_streams() {
    let gateway = Stub;
    assert!(matches!(
        poll_once(gateway.handle("", None, b"x")),
        Some(Err(Failure::Rejected(Rejection::Unauthenticated)))
    ));
    assert!(matches!(
        poll_once(gateway.handle("k", Some("i"), b"x")),
        Some(Err(Failure::Rejected(Rejection::DuplicateRequest)))
    ));
    let reply = poll_once(gateway.handle("k", None, b"x"));
    assert!(matches!(
        &reply,
        Some(Ok(Reply {
            status: 200,
            streaming: true,
            ..
        }))
    ));
    if let Some(Ok(mut reply)) = reply {
        assert_eq!(
            poll_once(reply.body.next_chunk()),
            Some(Some(Ok(b"x".to_vec())))
        );
        assert_eq!(poll_once(reply.body.next_chunk()), Some(None));
    }
}

#[test]
fn events_carry_account_and_actual_cost() {
    let held = Money {
        currency: Currency::Usd,
        minor_units: 10,
    };
    let unreadable = UsageUnreadable {
        request: RequestId(String::from("r")),
        account: AccountId(String::from("a")),
        key: KeyId(String::from("k")),
        held,
        at: UnixMillis(1),
    };
    let capped = SettlementCapped {
        request: unreadable.request.clone(),
        account: unreadable.account.clone(),
        key: unreadable.key.clone(),
        held,
        actual: ActualCost::Overflow,
        at: UnixMillis(2),
    };
    assert_eq!(capped.account, unreadable.account);
    assert_ne!(
        capped.actual,
        ActualCost::Priced(Money {
            currency: Currency::Usd,
            minor_units: 11,
        })
    );
}
