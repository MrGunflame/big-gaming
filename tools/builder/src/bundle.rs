use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Builder;

pub struct Bundler<W>
where
    W: Write,
{
    builder: Builder<GzEncoder<W>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    TarGz,
}

impl Format {
    pub const fn file_extension(&self) -> &'static str {
        match self {
            Self::TarGz => "tar.gz",
        }
    }
}

impl<W> Bundler<W>
where
    W: Write,
{
    pub fn new(format: Format, writer: W) -> Self {
        match format {
            Format::TarGz => {
                let encoder = GzEncoder::new(writer, Compression::best());
                let builder = Builder::new(encoder);
                Self { builder }
            }
        }
    }

    pub fn append<P>(&mut self, path: P, file: &mut File) -> io::Result<()>
    where
        P: AsRef<Path>,
    {
        self.builder.append_file(path, file)
    }
}
