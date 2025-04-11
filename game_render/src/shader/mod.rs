use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, OnceLock};

use notify::{RecursiveMode, Watcher};
use thiserror::Error;

use crate::backend::shader::{self, Shader};
use crate::backend::ShaderModule;

#[derive(Clone, Debug)]
pub struct ShaderConfig {
    pub source: ShaderSource,
    pub language: ShaderLanguage,
}

#[derive(Clone, Debug)]
pub enum ShaderSource {
    String(&'static str),
    File(PathBuf),
}

impl ShaderSource {
    fn load(&self) -> io::Result<Cow<'static, str>> {
        match self {
            Self::String(s) => Ok(Cow::Borrowed(&s)),
            Self::File(path) => std::fs::read_to_string(path).map(Cow::Owned),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ShaderLanguage {
    Wgsl,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(io::Error),
    #[error(transparent)]
    Compilation(shader::Error),
}

#[derive(Debug)]
pub struct ReloadableShaderSource {
    source: ShaderSource,
    cell: Arc<AtomicBool>,
    sources: Vec<ShaderSource>,
}

impl ReloadableShaderSource {
    pub fn new(source: ShaderSource) -> Self {
        Self {
            source,
            cell: Arc::new(AtomicBool::new(false)),
            sources: Vec::new(),
        }
    }

    pub fn has_changed(&self) -> bool {
        self.cell.swap(false, Ordering::SeqCst)
    }

    pub fn compile(&mut self) -> Result<ShaderModule, Error> {
        for source in self.sources.drain(..) {
            match source {
                ShaderSource::File(path) => {
                    FileWatcher::unregister(path, self.cell.clone());
                }
                _ => (),
            }
        }

        let sources = load_files(self.source.clone()).map_err(Error::Io)?;
        for source in sources.sources {
            match source {
                ShaderSource::File(path) => {
                    FileWatcher::register(path, self.cell.clone());
                }
                _ => (),
            }
        }

        let mut combined = String::new();
        for data in sources.data {
            for line in data.lines() {
                // Preprocessor macros start with #
                if line.starts_with("#") {
                    continue;
                }

                combined.push_str(line);
                combined.push('\n');
            }
        }

        let shader = Shader::from_wgsl(&combined).map_err(Error::Compilation)?;

        Ok(ShaderModule { shader })
    }
}

enum WatchEvent {
    Register(PathBuf, Arc<AtomicBool>),
    Unregister(PathBuf, Arc<AtomicBool>),
    Changed(PathBuf),
}

fn load_files(root: ShaderSource) -> io::Result<ShaderSources> {
    let root_dir = match &root {
        ShaderSource::File(path) => path.parent().map(|s| s.as_os_str()).unwrap_or_default(),
        _ => <&OsStr>::default(),
    };

    let mut files = Vec::new();
    let mut sources = Vec::new();
    let mut queue = vec![root.clone()];

    while let Some(src) = queue.pop() {
        let data = src.load()?;
        sources.push(src);

        for line in data.lines() {
            if line.starts_with("//") {
                continue;
            }

            let Some(path) = line.strip_prefix("#include") else {
                continue;
            };

            let mut file_path = PathBuf::from(root_dir);
            file_path.push(PathBuf::from(path.trim()));
            if !path.is_empty() {
                queue.push(ShaderSource::File(file_path));
            }
        }

        files.push(data);
    }

    Ok(ShaderSources {
        sources,
        data: files,
    })
}

struct ShaderSources {
    sources: Vec<ShaderSource>,
    data: Vec<Cow<'static, str>>,
}

struct FileWatcher;

impl FileWatcher {
    fn register(path: PathBuf, cell: Arc<AtomicBool>) {
        Self::get().send(WatchEvent::Register(path, cell)).ok();
    }

    fn unregister(path: PathBuf, cell: Arc<AtomicBool>) {
        Self::get().send(WatchEvent::Unregister(path, cell)).ok();
    }

    fn get() -> &'static mpsc::Sender<WatchEvent> {
        static WATCHER: OnceLock<mpsc::Sender<WatchEvent>> = OnceLock::new();

        WATCHER.get_or_init(|| {
            let (tx, rx) = mpsc::channel();

            let event_handler = {
                let tx = tx.clone();
                move |res: notify::Result<notify::Event>| match res {
                    Ok(event) => {
                        if event.need_rescan()
                            || event.kind.is_create()
                            || event.kind.is_modify()
                            || event.kind.is_remove()
                        {
                            for path in event.paths {
                                tx.send(WatchEvent::Changed(path)).ok();
                            }
                        }
                    }
                    Err(_) => (),
                }
            };

            let mut watcher = notify::recommended_watcher(event_handler).unwrap();

            let mut paths = HashMap::<_, Vec<_>>::new();

            std::thread::spawn(move || {
                while let Ok(event) = rx.recv() {
                    match event {
                        WatchEvent::Register(path, cell) => {
                            let add_to_watcher = !paths.contains_key(&path);
                            paths.entry(path.clone()).or_default().push(cell);

                            if add_to_watcher {
                                watcher.watch(&path, RecursiveMode::NonRecursive).ok();
                            }
                        }
                        WatchEvent::Unregister(path, cell) => {
                            let mut remove_from_watcher = false;
                            if let Some(cells) = paths.get_mut(&path) {
                                cells.retain(|c| !Arc::ptr_eq(c, &cell));
                                if cells.is_empty() {
                                    paths.remove(&path);
                                    remove_from_watcher = true;
                                }
                            }

                            if remove_from_watcher {
                                watcher.unwatch(&path).ok();
                            }
                        }
                        WatchEvent::Changed(path) => {
                            if let Some(cells) = paths.get(&path) {
                                for cell in cells {
                                    cell.store(true, Ordering::SeqCst);
                                }
                            }
                        }
                    }
                }
            });

            tx
        })
    }
}
