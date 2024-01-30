use std::sync::OnceLock;

static DEBUG_LAYERS_ENABLED: OnceLock<bool> = OnceLock::new();

pub fn debug_layers_enabled() -> bool {
    // We enable debug layers if the `debug-layers` feature is enabled AND
    // `RENDER_DEBUG_LAYERS` is not explicitly set to a falsy value.
    if cfg!(feature = "debug-layers") {
        *DEBUG_LAYERS_ENABLED.get_or_init(|| {
            if let Ok(val) = std::env::var("RENDER_DEGUG_LAYERS") {
                match val.as_str() {
                    "true" | "1" => true,
                    "false" | "0" => false,
                    _ => {
                        tracing::warn!("invalid value for RENDER_DEBUG_LAYERS env variable");
                        true
                    }
                }
            } else {
                true
            }
        })
    } else {
        false
    }
}
