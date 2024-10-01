use std::convert::Infallible;

use game_ui::runtime_v2::Widget;
use game_ui::style::Color;
use game_ui::widgets::{Svg, SvgData, SvgStyle};

const SVG_DATA: &[u8] =
    include_bytes!("../../../assets/fonts/FontAwesome/svgs/regular/lightbulb.svg");

pub struct TestSvg;

impl Widget for TestSvg {
    type Message = Infallible;

    fn render(&self, _ctx: &game_ui::runtime_v2::Context<Self>) -> game_ui::runtime_v2::View {
        let data = SvgData::from_bytes(SVG_DATA).unwrap();

        Svg::new(data, 128, 128)
            .style(SvgStyle {
                color: Some(Color::WHITE),
            })
            .into()
    }
}
