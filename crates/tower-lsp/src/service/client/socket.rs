//! Loopback connection to the language client.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::channel::mpsc::Receiver;
use futures::sink::Sink;
use futures::stream::{FusedStream, Stream, StreamExt};

use super::{ExitedError, PendingClientRequests, ServerState, State};
use tower_lsp_json_rpc::{RequestMessage, ResponseMessage};

/// A loopback channel for server-to-client communication.
#[derive(Debug)]
pub struct ClientSocket {
    pub(super) rx: Receiver<RequestMessage>,
    pub(super) pending: Arc<PendingClientRequests>,
    pub(super) state: Arc<ServerState>,
}

impl ClientSocket {
    /// Splits this `ClientSocket` into two halves capable of operating independently.
    ///
    /// The two halves returned implement the [`Stream`] and [`Sink`] traits, respectively.
    ///
    /// [`Stream`]: futures::Stream
    /// [`Sink`]: futures::Sink
    pub fn split(self) -> (ClientRequestStream, ClientResponseSink) {
        let ClientSocket { rx, pending, state } = self;
        (
            ClientRequestStream {
                rx,
                state: state.clone(),
            },
            ClientResponseSink { pending, state },
        )
    }
}

/// Yields a stream of pending server-to-client requests.
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct ClientRequestStream {
    rx: Receiver<RequestMessage>,
    state: Arc<ServerState>,
}

impl Stream for ClientRequestStream {
    type Item = RequestMessage;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.state.get() == State::Exited || self.rx.is_terminated() {
            Poll::Ready(None)
        } else {
            self.rx.poll_next_unpin(cx)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.rx.size_hint()
    }
}

impl FusedStream for ClientRequestStream {
    fn is_terminated(&self) -> bool {
        self.rx.is_terminated()
    }
}

/// Routes client-to-server responses back to the server.
#[derive(Debug)]
pub struct ClientResponseSink {
    pending: Arc<PendingClientRequests>,
    state: Arc<ServerState>,
}

impl Sink<ResponseMessage> for ClientResponseSink {
    type Error = ExitedError;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.state.get() == State::Exited {
            Poll::Ready(Err(ExitedError(())))
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn start_send(self: Pin<&mut Self>, response: ResponseMessage) -> Result<(), Self::Error> {
        self.pending.register_response(response);
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
