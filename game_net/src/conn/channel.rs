use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{FutureExt, Sink, Stream};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;

use crate::proto::Packet;

use super::ConnectionStream;

pub struct ChannelStream {
    tx: mpsc::Sender<Packet>,
    rx: mpsc::Receiver<Packet>,
    future: Option<Pin<Box<dyn Future<Output = Result<(), SendError<Packet>>> + Send + Sync>>>,
}

impl ChannelStream {
    pub fn new(tx: mpsc::Sender<Packet>, rx: mpsc::Receiver<Packet>) -> Self {
        Self {
            tx,
            rx,
            future: None,
        }
    }

    /// Returns two `ChannelStream`s connected to each other.
    pub fn pair() -> (Self, Self) {
        let (tx0, rx0) = mpsc::channel(4096);
        let (tx1, rx1) = mpsc::channel(4096);
        (Self::new(tx0, rx1), Self::new(tx1, rx0))
    }
}

impl ConnectionStream for ChannelStream {
    const IS_RELIABLE: bool = true;
    const IS_ORDERED: bool = true;
}

impl Stream for ChannelStream {
    type Item = Packet;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

impl Sink<Packet> for ChannelStream {
    type Error = SendError<Packet>;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match &mut self.future {
            Some(fut) => match fut.poll_unpin(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Ok(())) => {
                    self.future = None;
                    Poll::Ready(Ok(()))
                }
                Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
            },
            None => Poll::Ready(Ok(())),
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: Packet) -> Result<(), Self::Error> {
        let tx = self.tx.clone();
        self.future = Some(Box::pin(async move { tx.send(item).await }));
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
