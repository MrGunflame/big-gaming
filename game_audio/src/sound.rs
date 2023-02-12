use glam::Vec3;

#[derive(Copy, Clone, Debug)]
pub struct SoundFrame {
    pub origin: Option<Vec3>,
    pub loudness: f32,
}

impl SoundFrame {
    #[inline]
    pub fn origin(mut self, origin: Vec3) -> Self {
        self.origin = Some(origin);
        self
    }

    #[inline]
    pub fn loadness(mut self, loudness: f32) -> Self {
        self.loudness = loudness;
        self
    }
}

impl Default for SoundFrame {
    #[inline]
    fn default() -> Self {
        Self {
            origin: None,
            loudness: 1.0,
        }
    }
}
