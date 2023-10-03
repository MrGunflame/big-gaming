use crate::{Decode, Model};

#[derive(Clone, Debug)]
pub struct Parser {
    buffer: Vec<u8>,
}

impl Parser {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    // TODO: Implement a streaming parser.
    pub fn parse(&mut self, buf: &[u8], eof: bool) -> Result<Option<Model>, ()> {
        self.buffer.extend(buf);

        if eof {
            let model = Model::decode(&self.buffer[..])?;
            Ok(Some(model))
        } else {
            Ok(None)
        }
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}
