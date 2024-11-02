#[doc(hidden)]
pub use tracing;

/// Creates a new [`Span`] with the given `name` at the current callsite.
#[macro_export]
macro_rules! trace_span {
    ($name:expr) => {
        $crate::__trace_span_impl!($name)
    };
}

#[cfg(feature = "tracy")]
#[macro_export]
#[doc(hidden)]
macro_rules! __trace_span_impl {
    ($name:expr) => {{
        $crate::span::Span {
            inner: $crate::span::tracing::span!($crate::span::tracing::Level::TRACE, $name),
            _priv: (),
        }
    }};
}

#[cfg(not(feature = "tracy"))]
#[macro_export]
#[doc(hidden)]
macro_rules! __trace_span_impl {
    ($name:expr) => {{
        $crate::span::Span { _priv: () }
    }};
}

/// A `Span` represents a zone of time in which an operation is being run.
#[derive(Clone, Debug)]
pub struct Span {
    #[cfg(feature = "tracy")]
    #[doc(hidden)]
    pub inner: tracing::span::Span,
    #[doc(hidden)]
    pub _priv: (),
}

impl Span {
    /// Enters this span.
    pub fn entered(self) -> EnteredSpan {
        EnteredSpan {
            #[cfg(feature = "tracy")]
            _inner: self.inner.entered(),
        }
    }
}

/// A handle to a [`Span`] that is currently active.
#[derive(Debug)]
#[non_exhaustive]
pub struct EnteredSpan {
    #[cfg(feature = "tracy")]
    _inner: tracing::span::EnteredSpan,
}
