#[derive(Clone, Debug)]
pub struct Style {
    pub display: Display,
}

#[derive(Copy, Clone, Debug, Default)]
pub enum Display {
    #[default]
    Auto,
    Start,
    End,
    SpaceAround,
    SpaceBetween,
}
