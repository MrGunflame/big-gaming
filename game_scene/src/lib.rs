pub mod format;
mod loader;
mod model;
pub mod scene2;
mod spawner;

pub mod scene;

pub use crate::spawner::{SceneId, SceneSpawner};

#[cfg(feature = "gltf")]
mod gltf;

use format::SceneRoot;
use game_gltf::uri::Uri;
use game_gltf::GltfDecoder;
use game_tracing::trace_span;
use loader::LoadScene;
use scene::Scene;
use slotmap::DefaultKey;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

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

    // let scene = crate::format::from_slice(&buf).unwrap();

    let decoder = GltfDecoder::new(&buf).unwrap();
    let data = decoder.finish().unwrap();

    Some(data.load())
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
