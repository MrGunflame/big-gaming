use std::sync::OnceLock;

pub fn debug_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| load_bool_env("RENDER_DEBUG_LAYERS").unwrap_or(true))
}

pub fn gpuav_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| load_bool_env("RENDER_DEBUG_GPUAV").unwrap_or(false))
}

fn load_bool_env(name: &str) -> Option<bool> {
    match std::env::var(name).ok()?.as_str() {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => {
            tracing::warn!("invalid value for {} env variable", name);
            None
        }
    }
}
