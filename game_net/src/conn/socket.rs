use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::{FutureExt, Sink, Stream};
use tokio::sync::mpsc;

use crate::proto::{Encode, Packet};
use crate::Socket;

use super::ConnectionStream;

/// A stream from a UDP socket.
pub struct UdpSocketStream {
    stream: mpsc::Receiver<Packet>,
    socket: Arc<Socket>,
    peer: SocketAddr,
    future: Option<Pin<Box<dyn Future<Output = Result<usize, io::Error>> + Send + Sync>>>,
}

impl UdpSocketStream {
    pub fn new(stream: mpsc::Receiver<Packet>, socket: Arc<Socket>, peer: SocketAddr) -> Self {
        Self {
            socket,
            stream,
            peer,
            future: None,
        }
    }
}

impl ConnectionStream for UdpSocketStream {
    const IS_RELIABLE: bool = false;
    const IS_ORDERED: bool = false;
}

impl Stream for UdpSocketStream {
    type Item = Packet;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.stream.poll_recv(cx)
    }
}

impl Sink<Packet> for UdpSocketStream {
    type Error = io::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match &mut self.future {
            Some(fut) => match fut.poll_unpin(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(res) => {
                    self.future = None;
                    Poll::Ready(res.map(|_| ()))
                }
            },
            None => Poll::Ready(Ok(())),
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: Packet) -> Result<(), Self::Error> {
        let socket = self.socket.clone();
        let peer = self.peer;

        let mut buf = Vec::with_capacity(1500);
        item.encode(&mut buf).unwrap();
        let fut = Box::pin(async move { socket.send_to(&buf, peer).await });
        self.future = Some(fut);
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

impl Debug for UdpSocketStream {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("UdpSocketStream")
            .field("stream", &self.stream)
            .field("socket", &self.socket)
            .field("peer", &self.peer)
            .finish_non_exhaustive()
    }
}
