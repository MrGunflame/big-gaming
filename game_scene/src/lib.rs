mod format;
mod loader;
mod model;
pub mod scene2;
mod spawner;

pub mod scene;

pub use crate::spawner::SceneSpawner;

#[cfg(feature = "gltf")]
mod gltf;

use game_gltf::uri::Uri;
use game_gltf::GltfDecoder;
use game_model::{Decode, Model};
use game_tracing::trace_span;
use loader::LoadScene;
use scene::Scene;
use slotmap::DefaultKey;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SceneId(DefaultKey);

fn load_scene(path: PathBuf) -> Option<Scene> {
    let _span = trace_span!("load_scene").entered();

    let uri = Uri::from(path);

    let mut file = match File::open(uri.as_path()) {
        Ok(file) => file,
        Err(err) => {
            tracing::error!("failed to load scene from {:?}: {}", uri, err);
            return None;
        }
    };

    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();

    let scene = match detect_format(&buf) {
        Some(SceneFormat::Model) => {
            let data = match Model::decode(&buf[..]) {
                Ok(data) => data,
                Err(err) => {
                    tracing::error!("failed to load model: {:?}", err);
                    return None;
                }
            };

            data.load()
        }
        Some(SceneFormat::Gltf) => {
            let mut gltf = match GltfDecoder::new(&buf) {
                Ok(gltf) => gltf,
                Err(err) => {
                    tracing::error!("failed to load GLTF file: {}", err);
                    return None;
                }
            };

            while let Some(path) = gltf.pop_source() {
                let mut uri = uri.clone();
                uri.push(&path);
                let mut file = match std::fs::File::open(uri.as_path()) {
                    Ok(file) => file,
                    Err(err) => {
                        tracing::error!("failed to load file for GLTF: {}", err);
                        return None;
                    }
                };

                let mut buf = Vec::new();
                if let Err(err) = file.read_to_end(&mut buf) {
                    tracing::error!("failed to load file for GLTF: {}", err);
                    return None;
                }

                gltf.push_source(path, buf);
            }

            match gltf.finish() {
                Ok(gltf) => gltf.load(),
                Err(err) => {
                    tracing::error!("failed to load GLTF file: {}", err);
                    return None;
                }
            }
        }
        None => {
            tracing::error!("cannot detect scene format");
            return None;
        }
    };

    Some(scene)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum SceneFormat {
    Model,
    Gltf,
}

/// Attempt to detect the file format.
fn detect_format(buf: &[u8]) -> Option<SceneFormat> {
    if buf.starts_with(&game_model::MAGIC) {
        return Some(SceneFormat::Model);
    }

    // Starts with 'glTF' for binary format, or a JSON object.
    if buf.starts_with(&[b'g', b'l', b'T', b'F']) || buf.starts_with(&[b'{']) {
        return Some(SceneFormat::Gltf);
    }

    None
}
