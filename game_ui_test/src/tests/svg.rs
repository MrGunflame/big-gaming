use game_ui::reactive::Context;
use game_ui::style::Color;
use game_ui::widgets::{Svg, SvgData, SvgStyle, Widget};

const SVG_DATA: &[u8] =
    include_bytes!("../../../assets/fonts/FontAwesome/svgs/regular/lightbulb.svg");

pub fn svg(ctx: Context<()>) {
    let data = SvgData::from_bytes(SVG_DATA).unwrap();

    let svg = Svg::new(data, 128, 128).style(SvgStyle {
        color: Some(Color::WHITE),
    });
    svg.mount(&ctx);
}
