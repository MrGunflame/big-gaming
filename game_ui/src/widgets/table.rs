use game_tracing::trace_span;

use crate::runtime::Context;
use crate::style::{Border, Direction, Growth, Style};

use super::{Container, Widget};

#[derive(Clone, Debug)]
pub struct Table<H, D> {
    pub header: Vec<H>,
    pub rows: Vec<Vec<D>>,
    style: TableStyle,
}

impl<H, D> Table<H, D> {
    pub fn new(header: Vec<H>, rows: Vec<Vec<D>>) -> Self {
        Self {
            header,
            rows,
            style: TableStyle::default(),
        }
    }

    pub fn style(mut self, style: TableStyle) -> Self {
        self.style = style;
        self
    }
}

impl<H, D> Widget for Table<H, D>
where
    H: Widget,
    D: Widget,
{
    fn mount(mut self, parent: &Context) -> Context {
        let _span = trace_span!("Table::mount").entered();

        let table = Container::new()
            .style(Style {
                direction: Direction::Column,
                ..Default::default()
            })
            .mount(&parent);

        let style = Style {
            border: self.style.cell_border,
            growth: Growth::x(1.0),
            ..Default::default()
        };

        loop {
            if self.header.is_empty() {
                break;
            }

            let mut column = Container::new().mount(&table);

            let header = self.header.remove(0);
            header.mount(&column);

            for row in &mut self.rows {
                let cell = Container::new().style(style.clone()).mount(&column);
                if !row.is_empty() {
                    let elem = row.remove(0);
                    elem.mount(&cell);
                }
            }
        }

        table
    }
}

#[derive(Clone, Debug, Default)]
pub struct TableStyle {
    pub cell_border: Border,
}
