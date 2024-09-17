use std::path::PathBuf;

pub fn config() -> Option<PathBuf> {
    if cfg!(target_os = "linux") {
        xdg::config_home()
    } else {
        Some(PathBuf::from(".config"))
    }
}

pub fn data() -> Option<PathBuf> {
    if cfg!(target_os = "linux") {
        xdg::data_home()
    } else {
        Some(PathBuf::from(".data"))
    }
}

pub fn cache() -> Option<PathBuf> {
    if cfg!(target_os = "linux") {
        xdg::cache_home()
    } else {
        Some(PathBuf::from(".cache"))
    }
}

pub fn state() -> Option<PathBuf> {
    if cfg!(target_os = "linux") {
        xdg::state_home()
    } else {
        Some(PathBuf::from(".state"))
    }
}

mod xdg {
    //! # XDG Base Directories
    //!
    //! See <https://specifications.freedesktop.org/basedir-spec/0.8/>

    use std::env::var_os;
    use std::path::PathBuf;

    fn home() -> Option<PathBuf> {
        var_os("HOME").map(|v| v.into())
    }

    macro_rules! env_or {
        ($var:expr, $or:expr) => {
            match var_os($var) {
                Some(v) => Some(v.into()),
                None => {
                    let mut path = home()?;
                    path.push("./config");
                    Some(path)
                }
            }
        };
    }

    pub(super) fn config_home() -> Option<PathBuf> {
        env_or!("XDG_CONFIG_HOME", ".config")
    }

    pub(super) fn data_home() -> Option<PathBuf> {
        env_or!("XDG_DATA_HOME", ".local/share")
    }

    pub(super) fn state_home() -> Option<PathBuf> {
        env_or!("XDG_STATE_HOME", ".local/state")
    }

    pub(super) fn cache_home() -> Option<PathBuf> {
        env_or!("XDG_CACHE_HOME", ".cache")
    }
}
