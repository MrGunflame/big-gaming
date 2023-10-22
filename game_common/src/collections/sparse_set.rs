pub struct Key(usize);

#[derive(Clone, Debug)]
pub struct SparseSet<T> {
    sparse: Vec<usize>,
    dense: Vec<T>,
}

impl<T> SparseSet<T> {
    pub const fn new() -> Self {
        Self {
            sparse: Vec::new(),
            dense: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            sparse: Vec::with_capacity(capacity),
            dense: Vec::with_capacity(capacity),
        }
    }

    pub fn get(&self, key: Key) -> Option<&T> {
        let index = *self.sparse.get(key.0)?;
        let value = self.dense.get(index).unwrap();
        Some(value)
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        let index = *self.sparse.get(key.0)?;
        let value = self.dense.get_mut(index).unwrap();
        Some(value)
    }

    pub fn remove(&mut self, key: Key) -> Option<T> {
        let index = *self.sparse.get(key.0)?;
        let value = self.dense.swap_remove(index);
        Some(value)
    }

    pub fn insert(&mut self, value: T) -> Key {
        let dense_index = self.dense.len();
        self.dense.push(value);

        let index = self.sparse.len();
        self.sparse.push(dense_index);

        Key(index)
    }
}
