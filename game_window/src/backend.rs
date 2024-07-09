use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use winit::event_loop::EventLoop;

pub(crate) const DEFAULT_BACKEND: Backend = Backend(
    // Keep using X11 until the bug causing render blocking on
    // multiple surfaces are fixed.
    // https://github.com/MrGunflame/big-gaming/issues/220
    #[cfg(target_family = "unix")]
    Inner::X11,
    #[cfg(target_family = "windows")]
    Inner::Windows,
);

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct Backend(Inner);

impl Backend {
    pub(crate) const fn supports_locked_cursor(self) -> bool {
        match self.0 {
            Inner::Unknown => true,
            #[cfg(target_family = "unix")]
            Inner::Wayland => true,
            #[cfg(target_family = "unix")]
            Inner::X11 => false,
            #[cfg(target_family = "windows")]
            Inner::Windows => false,
        }
    }

    pub(crate) fn is_wayland(&self) -> bool {
        #[cfg(target_family = "unix")]
        if self.0 == Inner::Wayland {
            return true;
        }

        false
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
enum Inner {
    #[default]
    Unknown,
    #[cfg(target_family = "unix")]
    X11,
    #[cfg(target_family = "unix")]
    Wayland,
    #[cfg(target_family = "windows")]
    Windows,
}

impl From<&EventLoop<()>> for Backend {
    fn from(event_loop: &EventLoop<()>) -> Self {
        #[cfg(target_family = "unix")]
        {
            {
                use winit::platform::x11::EventLoopWindowTargetExtX11;

                if event_loop.is_x11() {
                    return Self(Inner::X11);
                }
            }

            {
                use winit::platform::wayland::EventLoopWindowTargetExtWayland;

                if event_loop.is_wayland() {
                    return Self(Inner::Wayland);
                }
            }
        }

        #[cfg(target_family = "windows")]
        {
            return Self(Inner::Windows);
        }

        // This is only dead code on some platforms.
        #[allow(unreachable_code)]
        Self(Inner::Unknown)
    }
}

impl FromStr for Backend {
    type Err = InvalidBackend;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            #[cfg(target_family = "unix")]
            "wayland" => Ok(Self(Inner::Wayland)),
            #[cfg(target_family = "unix")]
            "x11" => Ok(Self(Inner::X11)),
            #[cfg(target_family = "windows")]
            "windows" => Ok(Self(Inner::Windows)),
            _ => Err(InvalidBackend),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InvalidBackend;

impl Display for InvalidBackend {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("invalid backend")
    }
}

impl std::error::Error for InvalidBackend {}
