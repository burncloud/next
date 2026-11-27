use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};

use core::cell::Cell;

use identity::model::{Admission, KeyLimit, Principal, RequestsPerMinute};
use identity::ports::{ApiKeys, RateLimiter};
use kernel::model::{AccountId, KeyId, UnixMillis};

fn poll_once<T>(future: impl Future<Output = T>) -> Option<T> {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => Some(value),
        Poll::Pending => None,
    }
}

struct Stub;

impl ApiKeys for Stub {
    type Error = ();

    fn authenticate(
        &self,
        key: &str,
    ) -> impl Future<Output = Result<Option<Principal>, Self::Error>> + Send {
        let principal = (key == "k").then(|| Principal {
            account: AccountId(String::from("a")),
            key: KeyId(String::from("k")),
        });
        core::future::ready(Ok(principal))
    }

    fn limit(
        &self,
        key: &KeyId,
    ) -> impl Future<Output = Result<Option<KeyLimit>, Self::Error>> + Send {
        let limit = (key.0 == "k").then_some(KeyLimit {
            spend: None,
            requests_per_minute: None,
        });
        core::future::ready(Ok(limit))
    }
}

#[test]
fn api_keys_are_async() {
    let keys = Stub;
    assert_eq!(poll_once(keys.authenticate("x")), Some(Ok(None)));
    assert_eq!(
        poll_once(keys.authenticate("k")),
        Some(Ok(Some(Principal {
            account: AccountId(String::from("a")),
            key: KeyId(String::from("k")),
        })))
    );
    assert_eq!(
        poll_once(keys.limit(&KeyId(String::from("x")))),
        Some(Ok(None))
    );
    assert_eq!(
        poll_once(keys.limit(&KeyId(String::from("k")))),
        Some(Ok(Some(KeyLimit {
            spend: None,
            requests_per_minute: None,
        })))
    );
}

struct Counter {
    admitted: Cell<u32>,
}

impl RateLimiter for Counter {
    type Error = ();

    fn admit(
        &self,
        key: &KeyId,
        limit: RequestsPerMinute,
        now: UnixMillis,
    ) -> impl Future<Output = Result<Admission, Self::Error>> + Send {
        let result = if key.0.is_empty() || now.0 == 0 {
            Err(())
        } else if self.admitted.get() < limit.0 {
            self.admitted.set(self.admitted.get() + 1);
            Ok(Admission::Allowed)
        } else {
            Ok(Admission::Throttled)
        };
        core::future::ready(result)
    }
}

#[test]
fn rate_limiter_is_async() {
    let limiter = Counter {
        admitted: Cell::new(0),
    };
    let key = KeyId(String::from("k"));
    let limit = RequestsPerMinute(2);
    let now = UnixMillis(1);
    assert_eq!(
        poll_once(limiter.admit(&key, limit, now)),
        Some(Ok(Admission::Allowed))
    );
    assert_eq!(
        poll_once(limiter.admit(&key, limit, now)),
        Some(Ok(Admission::Allowed))
    );
    assert_eq!(
        poll_once(limiter.admit(&key, limit, now)),
        Some(Ok(Admission::Throttled))
    );
    assert_eq!(limiter.admitted.get(), 2);
    assert_eq!(
        poll_once(limiter.admit(&KeyId(String::new()), limit, now)),
        Some(Err(()))
    );
    assert_eq!(limiter.admitted.get(), 2);
}
