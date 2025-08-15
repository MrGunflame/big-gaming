mod spirv;
mod wgsl;

use std::borrow::Cow;
use std::io;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, OnceLock};

use bitflags::bitflags;
use hashbrown::HashMap;
use notify::{RecursiveMode, Watcher};
use thiserror::Error;

use crate::backend::{DescriptorType, ShaderStage};

#[derive(Clone, Debug)]
pub enum Shader {
    Wgsl(wgsl::Shader),
    Spirv(spirv::Module),
}

impl Shader {
    pub fn from_wgsl(s: &str) -> Result<Self, wgsl::Error> {
        wgsl::Shader::from_wgsl(s).map(Self::Wgsl)
    }

    pub fn from_spirv(b: &[u8]) -> Result<Self, spirv::Error> {
        spirv::Module::new(b).map(Self::Spirv)
    }

    pub fn instantiate(&self, options: &Options<'_>) -> ShaderInstance<'_> {
        let s = match self {
            Self::Wgsl(s) => ShaderInstance::Wgsl(s.instantiate(options)),
            Self::Spirv(s) => ShaderInstance::Spirv(s.instantiate(options).unwrap()),
        };
        std::fs::write("/tmp/x", bytemuck::cast_slice(&s.to_spirv()));
        s
    }

    pub fn bindings(&self) -> Vec<ShaderBinding> {
        match self {
            Self::Wgsl(s) => s.bindings(),
            Self::Spirv(s) => s.bindings(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Options<'a> {
    pub entry_point: &'a str,
    pub stage: ShaderStage,
    pub bindings: HashMap<BindingLocation, BindingInfo>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BindingLocation {
    pub group: u32,
    pub binding: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct ShaderBinding {
    pub group: u32,
    pub binding: u32,
    pub kind: DescriptorType,
    pub access: ShaderAccess,
    /// If the binding point is an binding array this will be greater than 1.
    ///
    /// This is always 1 for non-array types.
    ///
    /// `None` indicates that the count is still undefined and needs to specialized on
    /// instantiation.
    pub count: Option<NonZeroU32>,
}

impl ShaderBinding {
    pub fn location(&self) -> BindingLocation {
        BindingLocation {
            group: self.group,
            binding: self.binding,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BindingInfo {
    pub count: NonZeroU32,
}

#[derive(Clone, Debug)]
pub enum ShaderInstance<'a> {
    Wgsl(wgsl::ShaderInstance<'a>),
    Spirv(spirv::Instance),
}

impl<'a> ShaderInstance<'a> {
    pub fn bindings(&self) -> &[ShaderBinding] {
        match self {
            Self::Wgsl(s) => s.bindings(),
            Self::Spirv(s) => s.bindings(),
        }
    }

    pub fn to_spirv(&self) -> Vec<u32> {
        match self {
            Self::Wgsl(s) => s.to_spirv(),
            Self::Spirv(s) => s.to_spirv(),
        }
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
    pub struct ShaderAccess: u8 {
        /// The resource will be read from.
        const READ = 1 << 0;
        /// The resource will be written to.
        const WRITE = 1 << 1;
    }
}

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
    Slang,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(io::Error),
    #[error(transparent)]
    Wgsl(wgsl::Error),
    #[error(transparent)]
    Slang(slangc::Error),
    #[error(transparent)]
    Spirv(spirv::Error),
}

#[derive(Debug)]
pub struct ReloadableShaderSource {
    config: ShaderConfig,
    cell: Arc<AtomicBool>,
    sources: Vec<ShaderSource>,
}

impl ReloadableShaderSource {
    pub fn new(config: ShaderConfig) -> Self {
        Self {
            config,
            cell: Arc::new(AtomicBool::new(false)),
            sources: Vec::new(),
        }
    }

    pub fn has_changed(&self) -> bool {
        self.cell.swap(false, Ordering::SeqCst)
    }

    pub fn compile(&mut self) -> Result<Shader, Error> {
        for source in self.sources.drain(..) {
            match source {
                ShaderSource::File(path) => {
                    FileWatcher::unregister(path, self.cell.clone());
                }
                _ => (),
            }
        }

        let shader = match self.config.language {
            ShaderLanguage::Wgsl => {
                let sources = wgsl::load_files(self.config.source.clone()).map_err(Error::Io)?;

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

                let shader = Shader::from_wgsl(&combined).map_err(Error::Wgsl)?;

                for source in sources.sources {
                    match source {
                        ShaderSource::File(path) => {
                            FileWatcher::register(path, self.cell.clone());
                        }
                        _ => (),
                    }
                }

                shader
            }
            ShaderLanguage::Slang => {
                let path = match &self.config.source {
                    ShaderSource::File(path) => path,
                    _ => panic!("Only file sources are supported for slang shaders"),
                };

                let bytes = slangc::compile(&path, slangc::OptLevel::Max).map_err(Error::Slang)?;
                let module = spirv::Module::new(&bytes).map_err(Error::Spirv)?;

                let sources = slangc::load_imported_files(&path).map_err(Error::Slang)?;
                for source in sources {
                    FileWatcher::register(source, self.cell.clone());
                }

                Shader::Spirv(module)
            }
        };

        Ok(shader)
    }
}

enum WatchEvent {
    Register(PathBuf, Arc<AtomicBool>),
    Unregister(PathBuf, Arc<AtomicBool>),
    Changed(PathBuf),
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
