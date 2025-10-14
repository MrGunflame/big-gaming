use std::collections::HashMap;
use std::ops::Deref;

use parking_lot::{Mutex, RwLock, RwLockReadGuard};

use crate::api::{CommandQueue, Pipeline};
use crate::backend::TextureFormat;
use crate::shader::{ReloadableShaderSource, Shader, ShaderConfig, ShaderInstance};

/// A cache for pipelines with different [`TextureFormat`].
#[derive(Debug)]
pub struct PipelineCache<T> {
    pub builder: T,
    pipelines: RwLock<HashMap<TextureFormat, Pipeline>>,
    shaders: RwLock<Vec<ReloadableShaderSource>>,
    compiled_shaders: Mutex<Vec<Shader>>,
}

impl<T> PipelineCache<T> {
    /// Creates a new `PipelineCache`.
    pub fn new(builder: T, shaders: Vec<ShaderConfig>) -> Self {
        Self {
            pipelines: RwLock::new(HashMap::new()),
            builder,
            compiled_shaders: Mutex::new(Vec::with_capacity(shaders.len())),
            shaders: RwLock::new(
                shaders
                    .into_iter()
                    .map(|shader| ReloadableShaderSource::new(shader))
                    .collect(),
            ),
        }
    }
}

impl<T> PipelineCache<T>
where
    T: PipelineBuilder,
{
    /// Returns the pipeline with the requested [`TextureFormat`].
    ///
    /// If the pipeline with the requested for [`TextureFormat`] does not exist a new pipeline will
    /// be created using the [`PipelineBuilder`].
    pub fn get<'a, 'b>(
        &'a self,
        queue: &'b CommandQueue<'_>,
        format: TextureFormat,
    ) -> PipelineRef<'a> {
        let mut pipelines = self.pipelines.read();

        // If any shader in the pipeline has changed drop all current
        // pipelines and recompile shaders from the updated sources.
        {
            let shaders = self.shaders.read();
            for shader in &*shaders {
                if shader.has_changed() {
                    tracing::info!("reloading shader");

                    drop(pipelines);
                    {
                        let mut pipelines = self.pipelines.write();
                        pipelines.clear();
                    }
                    pipelines = self.pipelines.read();

                    break;
                }
            }
        }

        // Note that this case will likely happen very rarely under normal
        // operation. (Likely less than a dozen times)
        // It is therefore not a big deal to block in this function while
        // a pipeline is being built.
        if !pipelines.contains_key(&format) {
            drop(pipelines);

            let mut shaders = self.shaders.write();
            let mut compiled_shaders = self.compiled_shaders.lock();
            if compiled_shaders.is_empty() {
                for shader in &mut *shaders {
                    match shader.compile() {
                        Ok(module) => compiled_shaders.push(module),
                        Err(err) => {
                            tracing::error!("failed to compile shader: {}", err);
                            panic!("failed to compile inital shader");
                        }
                    }
                }
            } else {
                for (shader, compiled) in shaders.iter_mut().zip(&mut *compiled_shaders) {
                    match shader.compile() {
                        Ok(module) => *compiled = module,
                        Err(err) => {
                            tracing::error!("failed to compile shader: {}", err);
                        }
                    }
                }
            }

            debug_assert_eq!(compiled_shaders.len(), shaders.len());

            {
                let mut pipelines = self.pipelines.write();
                let pipeline = self.builder.build(queue, &compiled_shaders, format);
                pipelines.insert(format, pipeline);
            }

            pipelines = self.pipelines.read();
        }

        match pipelines.get(&format) {
            Some(pipeline) => {
                // SAFETY: The pipeline is valid to the accessed as long
                // as the mutex is locked.
                // To guarantee this the mutex guard is attached to the
                // returned struct.
                let pipeline = unsafe { core::mem::transmute::<&'_ _, &'a _>(pipeline) };
                PipelineRef {
                    pipeline,
                    _pipelines: pipelines,
                }
            }
            None => unreachable!(),
        }
    }
}

/// A reference to the pipeline inside a [`PipelineCache`].
///
/// Returned by [`PipelineCache::get`].
#[derive(Debug)]
pub struct PipelineRef<'a> {
    // The reference to the `pipeline` is attached to the read guard
    // of the cache.
    // The reference must not be acessed after the read guard was dropped.
    pipeline: &'a Pipeline,
    // This must come at the end of the struct and guarantees that the
    // reference to the `pipeline` is invalidated once the struct is dropped.
    _pipelines: RwLockReadGuard<'a, HashMap<TextureFormat, Pipeline>>,
}

impl<'a> Deref for PipelineRef<'a> {
    type Target = Pipeline;

    fn deref(&self) -> &Self::Target {
        self.pipeline
    }
}

pub trait PipelineBuilder {
    /// Returns a new pipeline with the requested [`TextureFormat`].
    fn build(
        &self,
        queue: &CommandQueue<'_>,
        shaders: &[Shader],
        format: TextureFormat,
    ) -> Pipeline;
}
