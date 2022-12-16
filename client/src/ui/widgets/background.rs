use bevy_egui::egui::{Align, Color32, InnerResponse, Layout, Rounding, Sense, Stroke, Ui};
use bevy_egui::egui::{Pos2, Rect};

use self::private::Sealed;

const COLOR: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 120);

pub trait UiExt: Sealed {
    fn transparent_background<R>(
        &mut self,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R>;
}

impl UiExt for Ui {
    fn transparent_background<R>(
        &mut self,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let space = self.available_size();

        let (resp, painter) = self.allocate_painter(
            space,
            Sense {
                drag: false,
                click: false,
                focusable: false,
            },
        );

        let rect = Rect {
            min: Pos2::new(0.0, 0.0),
            max: Pos2::new(space.x, space.y),
        };

        painter.rect(rect, Rounding::none(), COLOR, Stroke::none());

        let mut ui = self.child_ui(rect, Layout::left_to_right(Align::TOP));

        InnerResponse {
            response: resp,
            inner: add_contents(&mut ui),
        }
    }
}

#[doc(hidden)]
impl Sealed for Ui {}

mod private {
    pub trait Sealed {}
}
