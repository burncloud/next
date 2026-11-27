use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};

use identity::model::{KeyLimit, Principal};
use identity::ports::ApiKeys;
use kernel::model::{AccountId, KeyId};

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
