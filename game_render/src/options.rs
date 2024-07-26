use bytemuck::{Pod, Zeroable};

#[derive(Clone, Debug, Default)]
pub struct MainPassOptions {
    pub shading: ShadingMode,
}

/// The shading mode of the main pipeline.
#[derive(Copy, Clone, Debug, Default)]
pub enum ShadingMode {
    /// The full shading pipeline.
    ///
    /// This mode enalbes full shading based on material properties and lighting.
    #[default]
    Full,
    /// Only render material albedo.
    Albedo,
    /// Only render the object normals and the material normal maps.
    Normal,
    /// Only render the object tangents.
    Tangent,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub(crate) struct MainPassOptionsEncoded {
    shading_mode: u32,
}

impl MainPassOptionsEncoded {
    pub(crate) fn new(options: &MainPassOptions) -> Self {
        Self {
            shading_mode: match options.shading {
                ShadingMode::Full => 0,
                ShadingMode::Albedo => 1,
                ShadingMode::Normal => 2,
                ShadingMode::Tangent => 3,
            },
        }
    }
}
