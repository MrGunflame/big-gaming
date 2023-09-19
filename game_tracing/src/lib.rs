use tracing_subscriber::layer::SubscriberExt;

pub mod world;

pub const IS_ENABLED: bool = true;

pub fn init() {
    if IS_ENABLED {
        tracing::subscriber::set_global_default(
            tracing_subscriber::registry().with(tracing_tracy::TracyLayer::new()),
        )
        .unwrap();
    }
}

#[macro_export]
macro_rules! trace_span {
    ($name:expr) => {{
        if game_tracing::IS_ENABLED {
            tracing::span!(tracing::Level::TRACE, $name)
        } else {
            tracing::Span::none()
        }
    }};
}
