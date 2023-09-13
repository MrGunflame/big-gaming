//! Dump a debug-representation of a [`LayoutTree`].

use std::fmt::Write;

use super::{ElementBody, Key, LayoutTree};

const WHITESPACE_WIDTH: u32 = 4;

impl LayoutTree {
    pub fn dump_debug(&self) -> String {
        let mut buf = String::new();

        for elem in &self.root {
            self.debug_elem(*elem, &mut buf, 0);
        }

        buf
    }

    fn debug_elem(&self, key: Key, buf: &mut String, depth: u32) {
        for _ in 0..depth * WHITESPACE_WIDTH {
            buf.push(' ');
        }

        let elem = self.elems.get(&key).unwrap();
        let layout = self.layouts.get(&key).unwrap();
        match elem.body {
            ElementBody::Container => buf.push_str("Container"),
            ElementBody::Image(_) => buf.push_str("Image "),
            ElementBody::Text(_) => buf.push_str("Text "),
        }

        write!(
            buf,
            "(key={:?} pos={}:{}, w={}, h={})",
            key, layout.position.x, layout.position.y, layout.width, layout.height
        )
        .unwrap();

        buf.push_str("{\n");

        let bounds = self.compute_bounds(key);

        for _ in 0..depth * WHITESPACE_WIDTH {
            buf.push(' ');
        }
        writeln!(
            buf,
            "bounds: {}:{} => {}:{}, computed: {}:{} => {}:{};",
            elem.style.bounds.min.x.to_pixels(self.size),
            elem.style.bounds.min.y.to_pixels(self.size),
            elem.style.bounds.max.x.to_pixels(self.size),
            elem.style.bounds.max.y.to_pixels(self.size),
            bounds.min.x,
            bounds.min.y,
            bounds.max.x,
            bounds.max.y,
        )
        .unwrap();

        // for _ in 0..depth * WHITESPACE_WIDTH  {
        //     buf.push(' ');
        // }
        // writeln!(
        //     buf,
        //     "padding: top={} bottom={} left={} right={};",
        //     elem.style.padding.top.to_pixels(self.size),
        //     elem.style.padding.bottom.to_pixels(self.size),
        //     elem.style.padding.left.to_pixels(self.size),
        //     elem.style.padding.right.to_pixels(self.size),
        // )
        // .unwrap();

        if let Some(children) = self.children.get(&key) {
            for child in children {
                self.debug_elem(*child, buf, depth + 1);
            }
        }

        for _ in 0..depth * WHITESPACE_WIDTH {
            buf.push(' ');
        }
        buf.push_str("}\n");
    }
}
