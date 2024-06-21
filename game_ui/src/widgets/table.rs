use crate::reactive::Context;
use crate::style::{Direction, Style};

use super::{Container, Text, Widget};

pub struct Table {
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Widget for Table {
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
            Text::new(header).mount(&column);

            for row in &self.rows {
                Text::new(&row[column_index]).mount(&column);
            }

            column = Container::new().mount(&table);
            column_index += 1;
        }

        table
    }
}
