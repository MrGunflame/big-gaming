use std::str::FromStr;

use thiserror::Error;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MimeType(&'static str);

macro_rules! define_mime_type {
    ($($typ:ident => $val:expr);*$(;)?) => {
        impl MimeType {
            $(
                pub const $typ: Self = Self($val);
            )*
        }

        impl FromStr for MimeType {
            type Err = InvalidMimeType;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $(
                        $val => Ok(Self::$typ),
                    )*
                    _ => Err(InvalidMimeType),
                }
            }
        }
    };
}

define_mime_type! {
    // Application
    APPLICATION_OCTET_STREAM => "application/octet-stream";

    // Image
    IMAGE_PNG => "image/png";
    IMAGE_BMP => "image/bmp";
    IMAGE_AVIF => "image/avif";
    IMAGE_GIF => "image/gif";
    IMAGE_JPEG => "image/jpeg";
    IMAGE_TIFF => "image/tiff";
    IMAGE_WEBP => "image/webp";
}

impl MimeType {
    pub fn is_image(self) -> bool {
        match self {
            val if val == Self::IMAGE_PNG => true,
            val if val == Self::IMAGE_BMP => true,
            val if val == Self::IMAGE_AVIF => true,
            val if val == Self::IMAGE_GIF => true,
            val if val == Self::IMAGE_JPEG => true,
            val if val == Self::IMAGE_TIFF => true,
            val if val == Self::IMAGE_WEBP => true,
            _ => false,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
#[error("invalid mime type")]
pub struct InvalidMimeType;
