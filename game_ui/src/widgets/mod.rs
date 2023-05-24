mod button;
mod container;
mod text;

pub use button::Button;
pub use container::Container;
pub use text::Text;

use crate::reactive::Scope;

pub trait Widget {
    type Properties;

    fn render(cx: &Scope, props: Self::Properties) -> Scope;
}

pub trait Component {
    type Properties;

    fn render(cx: &Scope, props: Self::Properties) -> Scope;
}
