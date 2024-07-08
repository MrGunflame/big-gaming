use game_ui::layout::LayoutTree;
use game_ui::primitive::Primitive;
use game_ui::style::{Bounds, Direction, Size, SizeVec2, Style};
use glam::UVec2;

const NUM_TABLES: usize = 5;
const NUM_COLUMNS: usize = 2;
const NUM_ENTRIES: usize = 10;

const ENTRY_SIZE: u32 = 10;

#[test]
fn test_table() {
    let mut tree = LayoutTree::new();

    let root = tree.push(
        None,
        create_node(Style {
            ..Default::default()
        }),
    );

    let mut tables = vec![];
    let mut columns = vec![];
    let mut entries = vec![];

    for _ in 0..NUM_TABLES {
        let table = tree.push(
            Some(root),
            create_node(Style {
                direction: Direction::Column,
                ..Default::default()
            }),
        );

        tables.push(table);

        let mut cols = vec![];
        let mut ents = vec![];
        for _ in 0..NUM_COLUMNS {
            let col = tree.push(Some(table), create_node(Style::default()));

            cols.push(col);
            ents.push(vec![]);
        }

        columns.push(cols.clone());

        for _ in 0..NUM_ENTRIES {
            for (index, parent) in cols.iter().enumerate() {
                let key = tree.push(
                    Some(*parent),
                    create_node(Style {
                        bounds: Bounds::exact(SizeVec2::splat(Size::Pixels(ENTRY_SIZE))),
                        ..Default::default()
                    }),
                );

                ents[index].push(key);
            }
        }

        entries.push(ents);
    }

    tree.resize(UVec2::MAX);
    tree.compute_layout();

    for (index, table) in tables.iter().enumerate() {
        let layout = tree.layout(*table).unwrap();

        let offset = (index * NUM_ENTRIES) as u32 * ENTRY_SIZE;
        assert_eq!(layout.position, UVec2::new(0, offset));
    }

    for (table_index, cols) in columns.iter().enumerate() {
        for (col_index, column) in cols.iter().enumerate() {
            let layout = tree.layout(*column).unwrap();

            let offset_y = (table_index * NUM_ENTRIES) as u32 * ENTRY_SIZE;
            let offset_x = col_index as u32 * ENTRY_SIZE;
            assert_eq!(layout.position, UVec2::new(offset_x, offset_y));
        }
    }

    for (table_index, table) in entries.iter().enumerate() {
        for (col_index, column) in table.iter().enumerate() {
            for (entry_index, entry) in column.iter().enumerate() {
                let layout = tree.layout(*entry).unwrap();

                let offset_y = ((table_index * NUM_ENTRIES) + entry_index) as u32 * ENTRY_SIZE;
                let offset_x = col_index as u32 * ENTRY_SIZE;
                assert_eq!(layout.position, UVec2::new(offset_x, offset_y));
            }
        }
    }
}

fn create_node(style: Style) -> Primitive {
    Primitive::from_style(style)
}
