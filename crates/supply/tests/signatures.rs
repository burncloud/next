use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};

use kernel::model::ModelId;

use supply::errors::{CatalogError, CatalogRejection};
use supply::model::{ChannelId, ModelRoute};
use supply::ports::Catalog;

fn poll_once<T>(future: impl Future<Output = T>) -> Option<T> {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => Some(value),
        Poll::Pending => None,
    }
}

struct Exact;

impl Catalog for Exact {
    type Error = ();

    fn lookup(
        &self,
        model: &ModelId,
    ) -> impl Future<Output = Result<ModelRoute, CatalogError<Self::Error>>> + Send {
        let route = if model.0 == "gpt-4o-mini" {
            Ok(ModelRoute {
                model: model.clone(),
                channel: ChannelId(String::from("ch-openai")),
            })
        } else {
            Err(CatalogError::Rejected(CatalogRejection::UnknownModel))
        };
        core::future::ready(route)
    }
}

fn rejection_code(reason: CatalogRejection) -> u8 {
    match reason {
        CatalogRejection::UnknownModel => 0,
    }
}

#[test]
fn lookup_returns_the_channel_for_an_exact_model() {
    let catalog = Exact;
    let model = ModelId(String::from("gpt-4o-mini"));
    assert_eq!(
        poll_once(catalog.lookup(&model)),
        Some(Ok(ModelRoute {
            model: model.clone(),
            channel: ChannelId(String::from("ch-openai")),
        }))
    );
    assert_ne!(
        ChannelId(String::from("ch-openai")),
        ChannelId(String::from("ch-other"))
    );
    assert_eq!(rejection_code(CatalogRejection::UnknownModel), 0);
    assert!(matches!(
        CatalogError::<()>::Storage(()),
        CatalogError::Storage(())
    ));
}

#[test]
fn lookup_rejects_unknown_prefix_and_case() {
    let catalog = Exact;
    let unknown = [
        ModelId(String::from("gpt-4o")),
        ModelId(String::from("gpt-4o-mini-2024")),
        ModelId(String::from("GPT-4o-mini")),
        ModelId(String::new()),
    ];
    for model in &unknown {
        assert_eq!(
            poll_once(catalog.lookup(model)),
            Some(Err(CatalogError::Rejected(CatalogRejection::UnknownModel)))
        );
    }
    assert_ne!(
        ModelId(String::from("gpt-4o-mini")),
        ModelId(String::from("GPT-4o-mini"))
    );
}
