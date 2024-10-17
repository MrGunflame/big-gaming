mod allocator;

pub use allocator::ProfiledAllocator;
pub use tracing_tracy::TracyLayer;
pub use tracy_client::Client;

#[doc(hidden)]
pub use tracing;

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

#[cfg_attr(all(not(miri), not(test)), global_allocator)]
static GLOBAL: ProfiledAllocator<std::alloc::System> = ProfiledAllocator::new(std::alloc::System);
