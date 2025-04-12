use game_window::windows::WindowId;

use crate::texture::RenderImageId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RenderTarget {
    /// Render to a window surface.
    Window(WindowId),
    /// Render to a GPU internal texture.
    Image(RenderImageId),
}

impl RenderTarget {
    /// Returns `true` if this `RenderTarget` is a `Window`.
    #[inline]
    pub const fn is_window(&self) -> bool {
        matches!(self, Self::Window(_))
    }

    /// Returns `true` if this `RenderTarget` is a `Image`.
    #[inline]
    pub const fn is_image(&self) -> bool {
        matches!(self, Self::Image(_))
    }

    /// Returns the underlying [`WindowId`] or `None` if this `RenderTarget` is not `Window`.
    #[inline]
    pub const fn as_window(&self) -> Option<&WindowId> {
        match self {
            Self::Window(window) => Some(window),
            Self::Image(_) => None,
        }
    }

    /// Returns the underlying [`RenderImageId`] or `None` if this `RenderTarget` is not `Image`.
    #[inline]
    pub const fn as_image(&self) -> Option<&RenderImageId> {
        match self {
            Self::Image(image) => Some(image),
            Self::Window(_) => None,
        }
    }
}

impl From<WindowId> for RenderTarget {
    #[inline]
    fn from(value: WindowId) -> Self {
        Self::Window(value)
    }
}

impl From<RenderImageId> for RenderTarget {
    #[inline]
    fn from(value: RenderImageId) -> Self {
        Self::Image(value)
    }
}
