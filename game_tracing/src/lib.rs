mod allocator;

pub use allocator::ProfiledAllocator;
pub use tracy_client::Client;

use tracing::Metadata;
use tracing_tracy::{Config, DefaultConfig, TracyLayer};

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

pub type ProfilingLayer = TracyLayer<ProfilerConfig>;

#[derive(Default)]
pub struct ProfilerConfig(DefaultConfig);

impl Config for ProfilerConfig {
    type Formatter = <DefaultConfig as Config>::Formatter;

    fn formatter(&self) -> &Self::Formatter {
        self.0.formatter()
    }

    fn format_fields_in_zone_name(&self) -> bool {
        false
    }

    fn stack_depth(&self, metadata: &Metadata<'_>) -> u16 {
        self.0.stack_depth(metadata)
    }

    fn on_error(&self, client: &Client, error: &'static str) {
        self.0.on_error(client, error);
    }
}

#[cfg_attr(all(not(miri), not(test)), global_allocator)]
static GLOBAL: ProfiledAllocator<std::alloc::System> = ProfiledAllocator::new(std::alloc::System);
