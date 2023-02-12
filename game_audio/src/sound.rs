#[derive(Copy, Clone, Debug)]
pub struct Sound {
    pub loudness: f32,
}

impl Default for Sound {
    fn default() -> Self {
        Self { loudness: 1.0 }
    }
}
