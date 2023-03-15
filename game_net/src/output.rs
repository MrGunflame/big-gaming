use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use std::time::{Duration, Instant};

use pin_project::pin_project;
use tokio::time::Sleep;

use crate::snapshot::{CommandQueue, ConnectionMessage};

#[derive(Debug)]
#[pin_project]
pub struct OutputBuffer {
    /// Client interpolation delay
    delay: Duration,
    queue: BinaryHeap<MessageSchedule>,
    sink: CommandQueue,
    #[pin]
    sleep: Option<Sleep>,
}

impl OutputBuffer {
    pub fn new(delay: Duration, sink: CommandQueue) -> Self {
        Self {
            delay,
            queue: BinaryHeap::new(),
            sink,
            sleep: None,
        }
    }

    pub fn push(&mut self, message: ConnectionMessage) {
        // TODO: Account for packet delay (RTT / 2).
        let delivery = Instant::now() + self.delay;

        self.queue.push(MessageSchedule { delivery, message });

        // Update the timer if the inserted message is the new root.
        if self.queue.peek().unwrap().delivery == delivery {
            self.sleep = Some(tokio::time::sleep_until(delivery.into()));
        }
    }

    pub fn poll_advance(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        let this = self.project();

        let Some(mut sleep) = this.sleep.as_pin_mut() else {
            return Poll::Ready(());
        };

        ready!(sleep.as_mut().poll(cx));

        let msg = this.queue.pop().unwrap();
        this.sink.push(msg.message);

        if let Some(msg) = this.queue.peek() {
            sleep.reset(msg.delivery.into());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

#[derive(Clone, Debug)]
struct MessageSchedule {
    /// The timestamp at which this command should be delivered.
    delivery: Instant,
    message: ConnectionMessage,
}

impl PartialEq for MessageSchedule {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.delivery == other.delivery
    }
}

impl Eq for MessageSchedule {}

impl PartialOrd for MessageSchedule {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MessageSchedule {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.delivery.cmp(&other.delivery)
    }
}
