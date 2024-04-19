use std::alloc::Layout;

#[derive(Debug)]
pub struct Allocator {
    max_size: usize,
    free_size: usize,
    chunks: Vec<Chunk>,
}

impl Allocator {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            free_size: max_size,
            chunks: vec![Chunk {
                start: 0,
                size: max_size,
            }],
        }
    }

    pub fn alloc(&mut self, layout: Layout) -> Result<Allocation, AllocationError> {
        for (index, chunk) in self.chunks.iter_mut().enumerate() {
            let offset = (chunk.start as *const ()).align_offset(layout.align());

            if chunk.size < layout.size() + offset {
                continue;
            }

            let start = chunk.start + offset;
            let size = layout.size();
            let end = start + size;

            // Region remaining at the end of the current allocation.
            let remaining = chunk.size - end;

            match (offset, remaining) {
                // The current allocation takes up the entire chunk.
                (0, 0) => {
                    self.chunks.remove(index);
                }
                // If the offset is non-zero we may still use the memory
                // until our current allocation occurs.
                (offset, 0) => {
                    chunk.size = offset;
                }
                // If there is free memory in the current chunk beyond our
                // current allocation we move our current chunk pointer
                // forward.
                (0, remaining) => {
                    chunk.start = end;
                    chunk.size = remaining;
                }
                // There is free memory before and after the current allocation.
                // We reuse the current chunk for the *before* allocation and
                // create a new chunk for the *after* allocation.
                (offset, remaining) => {
                    chunk.size = offset;
                    self.chunks.insert(
                        index,
                        Chunk {
                            start: end,
                            size: remaining,
                        },
                    );
                }
            }

            self.free_size -= size;
            return Ok(Allocation { ptr: start, size });
        }

        Err(AllocationError::OutOfMemory)
    }

    pub fn dealloc(&mut self, allocation: Allocation) {
        todo!()
    }

    pub fn max_size(&self) -> usize {
        self.max_size
    }

    pub fn free_size(&self) -> usize {
        self.free_size
    }

    pub fn grow(&mut self, new_max_size: usize) {
        let old_max_size = self.max_size;
        self.max_size = new_max_size;

        // Attempt to grow the last chunk in place.
        if let Some(chunk) = self.chunks.last_mut() {
            if chunk.start + chunk.size == old_max_size {
                chunk.size += new_max_size - old_max_size;
                return;
            }
        }

        self.chunks.push(Chunk {
            start: old_max_size,
            size: new_max_size,
        });
    }
}

#[derive(Copy, Clone, Debug)]
struct Chunk {
    start: usize,
    size: usize,
}

#[derive(Debug)]
pub struct Allocation {
    ptr: usize,
    size: usize,
}

impl Allocation {
    pub fn ptr(&self) -> usize {
        self.ptr
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AllocationError {
    OutOfMemory,
}
