use tracing_subscriber::layer::SubscriberExt;

pub mod world;

#[doc(hidden)]
pub use tracing;

pub fn init() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(tracing_tracy::TracyLayer::new()),
    )
    .unwrap();
}

#[macro_export]
macro_rules! trace_span {
    ($name:expr) => {{
        $crate::Span {
            inner: $crate::tracing::span!($crate::tracing::Level::TRACE, $name),
        }
    }};
}

#[derive(Clone, Debug)]
pub struct Span {
    #[doc(hidden)]
    pub inner: tracing::span::Span,
}

impl Span {
    pub fn entered(self) -> EnteredSpan {
        EnteredSpan {
            _inner: self.inner.entered(),
        }
    }
}

#[derive(Debug)]
pub struct EnteredSpan {
    _inner: tracing::span::EnteredSpan,
}
