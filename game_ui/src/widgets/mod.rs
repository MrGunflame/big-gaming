mod button;
mod text;

pub use button::Button;
pub use text::Text;

use crate::reactive::Scope;

pub trait Widget {
    type Properties;

    fn render(cx: &Scope, props: Self::Properties) -> Scope;
}
