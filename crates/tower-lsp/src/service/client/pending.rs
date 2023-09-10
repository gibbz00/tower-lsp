//! Types for tracking server-to-client JSON-RPC requests.

use std::fmt::{self, Debug, Formatter};
use std::future::Future;

use dashmap::{mapref::entry::Entry, DashMap};
use futures::channel::oneshot;
use tracing::warn;

use tower_lsp_json_rpc::{Id, ResponseMessage};

/// A hashmap containing pending client requests, keyed by request ID.
pub struct PendingClientRequests(DashMap<Id, Vec<oneshot::Sender<ResponseMessage>>>);

impl PendingClientRequests {
    /// Creates a new pending client requests map.
    pub fn new() -> Self {
        PendingClientRequests(DashMap::new())
    }

    /// Inserts the given response into the map.
    ///
    /// The corresponding `.wait()` future will then resolve to the given value.
    pub fn register_response(&self, r: ResponseMessage) {
        match r.id() {
            Id::Null => warn!("received response with request ID of `null`, ignoring"),
            id => match self.0.entry(id.clone()) {
                Entry::Vacant(_) => warn!("received response with unknown request ID: {}", id),
                Entry::Occupied(mut entry) => {
                    let tx = match entry.get().len() {
                        1 => entry.remove().remove(0),
                        // IMPROVEMENT: might be more reasonable to use a VecDequeue
                        _ => entry.get_mut().remove(0),
                    };

                    tx.send(r).expect("receiver already dropped");
                }
            },
        }
    }

    /// Marks the given request ID as pending and waits for its corresponding response to arrive.
    ///
    /// If the same request ID is being waited upon in multiple locations, then the incoming
    /// response will be routed to one of the callers in a first come, first served basis. To
    /// ensure correct routing of JSON-RPC requests, each identifier value used _must_ be unique.
    pub fn await_response(&self, id: Id) -> impl Future<Output = ResponseMessage> + Send + 'static {
        let (tx, rx) = oneshot::channel();

        match self.0.entry(id) {
            Entry::Vacant(entry) => {
                entry.insert(vec![tx]);
            }
            Entry::Occupied(mut entry) => {
                let txs = entry.get_mut();
                txs.reserve(1); // We assume concurrent waits are rare, so reserve one by one.
                txs.push(tx);
            }
        }

        async { rx.await.expect("sender already dropped") }
    }
}

impl Debug for PendingClientRequests {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        #[derive(Debug)]
        struct Waiters(usize);

        let iter = self
            .0
            .iter()
            .map(|e| (e.key().clone(), Waiters(e.value().len())));

        f.debug_map().entries(iter).finish()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn waits_for_client_response() {
        let pending = PendingClientRequests::new();

        let id = Id::Number(1);
        let wait_fut = pending.await_response(id.clone());

        let response = ResponseMessage::from_ok(id, json!({}));
        pending.register_response(response.clone());

        assert_eq!(wait_fut.await, response);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn routes_responses_in_fifo_order() {
        let pending = PendingClientRequests::new();

        let id = Id::Number(1);
        let wait_fut1 = pending.await_response(id.clone());
        let wait_fut2 = pending.await_response(id.clone());

        let foo = ResponseMessage::from_ok(id.clone(), json!("foo"));
        let bar = ResponseMessage::from_ok(id, json!("bar"));
        pending.register_response(bar.clone());
        pending.register_response(foo.clone());

        assert_eq!(wait_fut1.await, bar);
        assert_eq!(wait_fut2.await, foo);
    }
}
