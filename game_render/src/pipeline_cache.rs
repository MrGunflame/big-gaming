use std::collections::HashMap;
use std::ops::Deref;

use parking_lot::{RwLock, RwLockReadGuard};

use crate::api::{CommandQueue, Pipeline};
use crate::backend::TextureFormat;

/// A cache for pipelines with different [`TextureFormat`].
#[derive(Debug)]
pub struct PipelineCache<T> {
    pipelines: RwLock<HashMap<TextureFormat, Pipeline>>,
    builder: T,
}

impl<T> PipelineCache<T> {
    /// Creates a new `PipelineCache`.
    pub fn new(builder: T) -> Self {
        Self {
            pipelines: RwLock::new(HashMap::new()),
            builder,
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
        queue: &'b mut CommandQueue<'_>,
        format: TextureFormat,
    ) -> PipelineRef<'a> {
        let mut pipelines = self.pipelines.read();

        // Note that this case will likely happen very rarely under normal
        // operation. (Likely less than a dozen times)
        // It is therefore not a big deal to block in this function while
        // a pipeline is being built.
        if !pipelines.contains_key(&format) {
            drop(pipelines);

            {
                let mut pipelines = self.pipelines.write();
                let pipeline = self.builder.build(queue, format);
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
    fn build(&self, queue: &mut CommandQueue<'_>, format: TextureFormat) -> Pipeline;
}
