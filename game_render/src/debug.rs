#[inline]
pub fn debug_layers_enabled() -> bool {
    // Disable debug layers at compile time if the cfg is set.
    #[cfg(render_debug_layers_disable)]
    #[inline]
    fn inner() -> bool {
        false
    }

    #[cfg(not(render_debug_layers_disable))]
    fn inner() -> bool {
        use std::sync::OnceLock;

        static DEBUG_LAYERS_ENABLED: OnceLock<bool> = OnceLock::new();

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
    }

    inner()
}
