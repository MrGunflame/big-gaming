use std::cell::RefCell;
use std::fmt::{Debug, Write};

use tracing::field::{Field, Visit};
use tracing::span::Id;
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;
use tracy_client::Client;

thread_local! {
    // Spans are !Send so we can keep them on the current thread.
    // Spans are also ordered, so we can use a Vec instead of a HashMap.
    static LOCAL_SPANS: RefCell<Vec<(tracy_client::Span, u64)>> = RefCell::new(Vec::new());
    // To prevent allocator pressure we keep the string buffer alive
    // for each thread.
    static STRING_BUFFER: RefCell<String> = RefCell::new(String::new());
}

pub struct TracyLayer {
    client: Client,
}

impl TracyLayer {
    pub fn new() -> Self {
        Self {
            client: Client::start(),
        }
    }
}

impl<S> Layer<S> for TracyLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        STRING_BUFFER.with_borrow_mut(|buf| {
            let mut visitor = Visitor { buf };
            event.record(&mut visitor);

            if visitor.buf.len() > u16::MAX as usize {
                let prefix = "[TRUNCATED] ";
                visitor.buf.truncate(u16::MAX as usize - prefix.len() - 1);
                visitor.buf.insert_str(0, prefix);
            }

            self.client.message(&visitor.buf, 0);

            buf.clear();
        });
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else {
            return;
        };

        let file = span.metadata().file().unwrap_or("???");
        let line = span.metadata().line().unwrap_or(0);
        let name = span.name();

        let span = self
            .client
            .clone()
            .span_alloc(Some(name), "", file, line, 0);

        LOCAL_SPANS.with_borrow_mut(|spans| {
            spans.push((span, id.into_u64()));
        });
    }

    fn on_exit(&self, id: &Id, _ctx: Context<'_, S>) {
        LOCAL_SPANS.with_borrow_mut(|spans| {
            if let Some((span, span_id)) = spans.pop() {
                if span_id != id.into_u64() {
                    tracing::warn!("tracing spans are out of order!");
                }

                drop(span);
            }
        });
    }
}

#[derive(Debug)]
struct Visitor<'a> {
    buf: &'a mut String,
}

impl<'a> Visit for Visitor<'a> {
    fn record_str(&mut self, field: &Field, value: &str) {
        if self.buf.is_empty() {
            write!(self.buf, "{} = {}", field.name(), value).ok();
        } else {
            write!(self.buf, ", {} = {}", field.name(), value).ok();
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if self.buf.is_empty() {
            write!(self.buf, "{} = {:?}", field.name(), value).ok();
        } else {
            write!(self.buf, ", {} = {:?}", field.name(), value).ok();
        }
    }
}
