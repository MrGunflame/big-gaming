use crate::reactive::Context;
use crate::style::{Direction, Style};

use super::{Container, Widget};

pub struct Table<H, D> {
    pub header: Vec<H>,
    pub rows: Vec<Vec<D>>,
}

impl<H, D> Widget for Table<H, D>
where
    H: Widget,
    D: Widget,
{
    fn mount<T>(mut self, parent: &Context<T>) -> Context<()> {
        let table = Container::new()
            .style(Style {
                direction: Direction::Column,
                ..Default::default()
            })
            .mount(&parent);

        loop {
            if self.header.is_empty() {
                break;
            }

            let mut column = Container::new().mount(&table);

            let header = self.header.remove(0);
            header.mount(&column);

            for row in &mut self.rows {
                if row.is_empty() {
                    Container::new().mount(&column);
                } else {
                    let elem = row.remove(0);
                    elem.mount(&column);
                }
            }
        }

        table
    }
}
