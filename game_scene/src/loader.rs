use crate::scene::Scene;

pub trait LoadScene {
    fn load(self) -> Scene;
}
