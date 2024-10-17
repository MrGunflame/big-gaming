pub mod debug;
pub mod format;
mod loader;
mod model;
pub mod scene2;
mod spawner;

pub mod scene;

pub use crate::spawner::{InstanceId, SceneId, SceneSpawner};

#[cfg(feature = "gltf")]
mod gltf;

use game_gltf::uri::Uri;
use game_gltf::GltfDecoder;
use game_model::{Decode, Model};
use game_tracing::trace_span;
use loader::LoadScene;
use scene::Scene;
use thiserror::Error;

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug, Error)]
pub enum LoadError {
    #[cfg(feature = "gltf")]
    #[error("invalid gltf: {0}")]
    Gltf(game_gltf::Error),
    #[error("invalid model data: {0:?}")]
    Model(()),
    #[error("unknown format")]
    UnknownFormat,
    #[error(transparent)]
    Io(io::Error),
}

fn load_scene<P>(path: P) -> Result<Scene, LoadError>
where
    P: AsRef<Path>,
{
    let _span = trace_span!("load_scene").entered();

    let uri = Uri::from(path);

    let mut file = File::open(uri.as_path()).map_err(LoadError::Io)?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf).map_err(LoadError::Io)?;

    load_from_bytes(&buf)
}

fn load_from_bytes(buf: &[u8]) -> Result<Scene, LoadError> {
    match detect_format(&buf) {
        #[cfg(feature = "gltf")]
        Some(SceneFormat::Gltf) => {
            let decoder = GltfDecoder::new(&buf).map_err(LoadError::Gltf)?;
            let data = decoder.finish().map_err(LoadError::Gltf)?;
            Ok(data.load())
        }
        Some(SceneFormat::Model) => {
            let model = Model::decode(&buf[..]).map_err(LoadError::Model)?;
            Ok(model.load())
        }
        None => Err(LoadError::UnknownFormat),
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum SceneFormat {
    Model,
    #[cfg(feature = "gltf")]
    Gltf,
}

/// Attempt to detect the file format.
fn detect_format(buf: &[u8]) -> Option<SceneFormat> {
    if buf.starts_with(&game_model::MAGIC) {
        return Some(SceneFormat::Model);
    }

    // Starts with 'glTF' for binary format, or a JSON object.
    #[cfg(feature = "gltf")]
    if buf.starts_with(&[b'g', b'l', b'T', b'F']) || buf.starts_with(&[b'{']) {
        return Some(SceneFormat::Gltf);
    }

    None
}
