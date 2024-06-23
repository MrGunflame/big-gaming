use crate::reactive::Context;
use crate::style::{Direction, Style};

use super::{Container, Widget};

pub struct Table<H, D> {
    pub header: Vec<H>,
    pub rows: Vec<Vec<D>>,
}

impl<H, D> Widget for Table<H, D>
where
    // TODO: Remove Clone bounds.
    H: Widget + Clone,
    D: Widget + Clone,
{
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let table = Container::new()
            .style(Style {
                direction: Direction::Column,
                ..Default::default()
            })
            .mount(&parent);

        let mut column = Container::new().mount(&table);
        let mut column_index = 0;
        while column_index < self.header.len() {
            let header = &self.header[column_index];
            header.clone().mount(&column);

            for row in &self.rows {
                row[column_index].clone().mount(&column);
            }

            column = Container::new().mount(&table);
            column_index += 1;
        }

        table
    }
}
