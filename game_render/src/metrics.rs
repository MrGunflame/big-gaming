#[derive(Copy, Clone, Debug, Default)]
pub struct RenderMetrics {
    pub entities: u64,
    pub triangles: u64,
    pub directional_lights: u64,
    pub point_lights: u64,
    pub spot_lights: u64,
}
