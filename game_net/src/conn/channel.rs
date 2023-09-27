use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{FutureExt, Sink, Stream};
use tokio::sync::mpsc;

use crate::proto::Packet;

use super::ConnectionStream;

pub struct ChannelStream {
    tx: mpsc::Sender<Packet>,
    rx: mpsc::Receiver<Packet>,
    future: Option<Pin<Box<dyn Future<Output = ()> + Send + Sync>>>,
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
    type Error = Infallible;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match &mut self.future {
            Some(fut) => match fut.poll_unpin(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(res) => {
                    self.future = None;
                    Poll::Ready(Ok(()))
                }
            },
            None => Poll::Ready(Ok(())),
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: Packet) -> Result<(), Self::Error> {
        let tx = self.tx.clone();
        self.future = Some(Box::pin(async move {
            tx.send(item).await;
        }));
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
