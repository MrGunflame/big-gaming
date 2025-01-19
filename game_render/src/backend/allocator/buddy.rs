use std::alloc::Layout;
use std::collections::VecDeque;

use game_tracing::trace_span;
use slab::Slab;

use super::{Allocator, Region};

#[derive(Clone, Debug)]
pub struct BuddyAllocator {
    region: Region,
    blocks: Slab<Block>,
    root: usize,
}

impl BuddyAllocator {
    pub fn new(region: Region) -> Self {
        assert!(region.size.is_power_of_two());

        let mut blocks = Slab::new();
        let root = blocks.insert(Block {
            offset: 0,
            size: region.size,
            state: State::Free,
            parent: None,
        });

        Self {
            region,
            blocks,
            root,
        }
    }
}

impl Allocator for BuddyAllocator {
    fn alloc(&mut self, layout: Layout) -> Option<Region> {
        let _span = trace_span!("BuddyAllocator::alloc").entered();

        let size = layout.size().next_power_of_two();
        debug_assert_ne!(size, 0);

        let mut queue = VecDeque::new();
        queue.push_back(self.root);

        while let Some(index) = queue.pop_front() {
            let block = &mut self.blocks[index];

            if size > block.size {
                continue;
            }

            match block.state {
                State::Free => (),
                State::Used => continue,
                State::Split { left, right } => {
                    // Walk down the left subtree first.
                    // This will keep smaller allocations on the left side.
                    queue.push_back(left);
                    queue.push_back(right);
                    continue;
                }
            }

            // Block is as small as possible for the allocation.
            // Note that this will always match at some point
            // since both `block.size` and `size` are power-of-twos.
            if block.size == size {
                block.state = State::Used;
                return Some(Region {
                    offset: block.offset,
                    size: block.size,
                });
            }

            // Split the block into two equally sized blocks.
            let left = Block {
                offset: block.offset,
                size: block.size / 2,
                state: State::Free,
                parent: Some(index),
            };
            let right = Block {
                offset: block.offset + block.size / 2,
                size: block.size / 2,
                state: State::Free,
                parent: Some(index),
            };

            let left = self.blocks.insert(left);
            let right = self.blocks.insert(right);

            // Mark the block as split.
            let block = &mut self.blocks[index];
            block.state = State::Split { left, right };

            // Since the parent block that we just split is bigger
            // than the requested size, either of the split blocks
            // will be big enough for `size`.
            // This means we only need to walk down the left side.
            debug_assert!(block.size >= size);
            queue.push_back(left);
        }

        None
    }

    unsafe fn dealloc(&mut self, region: Region) {
        let _span = trace_span!("BuddyAllocator::dealloc").entered();

        let (mut index, _) = self
            .blocks
            .iter()
            .find(|(_, block)| block.offset == region.offset && block.size == region.size)
            .unwrap();

        // Mark the deallocated block as free.
        let block = &mut self.blocks[index];
        debug_assert!(matches!(block.state, State::Used));
        block.state = State::Free;

        // Attempt to merge the freed block.
        let mut block = &self.blocks[index];
        while let Some(parent_index) = block.parent {
            let parent = &self.blocks[parent_index];

            let (left, right) = match parent.state {
                State::Split { left, right } => (left, right),
                _ => unreachable!(),
            };

            let other = if index == left { right } else { left };
            debug_assert_ne!(index, other);

            // If our boddy is not free we cannot merge any further.
            if !self.blocks[other].state.is_free() {
                break;
            }

            // Merge left and right back into parent.
            self.blocks.remove(left);
            self.blocks.remove(right);

            let parent = &mut self.blocks[parent_index];
            parent.state = State::Free;

            index = parent_index;
            block = &self.blocks[parent_index];
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Block {
    size: usize,
    offset: usize,
    state: State,
    parent: Option<usize>,
}

#[derive(Copy, Clone, Debug)]
enum State {
    Free,
    Used,
    Split { left: usize, right: usize },
}

impl State {
    const fn is_free(&self) -> bool {
        matches!(self, Self::Free)
    }
}

#[cfg(test)]
mod tests {
    use std::alloc::Layout;

    use crate::backend::allocator::{Allocator, Region};

    use super::BuddyAllocator;

    #[test]
    fn buddy_allocator_alloc() {
        let mut allocator = BuddyAllocator::new(Region::new(0, 128));
        let region = allocator
            .alloc(Layout::from_size_align(8, 1).unwrap())
            .unwrap();
        assert_eq!(region.size, 8);
    }

    #[test]
    fn buddy_allocator_alloc_no_overlap() {
        let mut allocator = BuddyAllocator::new(Region::new(0, 128 * 128));

        let mut regions = Vec::new();
        for _ in 0..128 {
            let region = allocator
                .alloc(Layout::from_size_align(128, 1).unwrap())
                .unwrap();
            regions.push(region);
        }

        if let Some((lhs, rhs)) = find_overlapping_regions(&regions) {
            panic!("regions overlap: {:?}, {:?}", lhs, rhs);
        }
    }

    #[test]
    fn buddy_allocator_alloc_and_dealloc() {
        let mut allocator = BuddyAllocator::new(Region::new(0, 128 * 128));

        let mut regions = Vec::new();
        for _ in 0..128 {
            let region = allocator
                .alloc(Layout::from_size_align(128, 1).unwrap())
                .unwrap();
            regions.push(region);
        }

        // Dealloc half the allocations.
        for region in regions.drain(..64) {
            unsafe {
                allocator.dealloc(region);
            }
        }

        // Realloc the dealloced allocations.
        let mut regions = Vec::new();
        for _ in 0..64 {
            let region = allocator
                .alloc(Layout::from_size_align(128, 1).unwrap())
                .unwrap();
            regions.push(region);
        }

        if let Some((lhs, rhs)) = find_overlapping_regions(&regions) {
            panic!("regions overlap: {:?}, {:?}", lhs, rhs);
        }
    }

    #[test]
    fn buddy_allocator_dealloc_all() {
        let mut allocator = BuddyAllocator::new(Region::new(0, 128 * 128));

        let mut regions = Vec::new();
        for _ in 0..128 {
            let region = allocator
                .alloc(Layout::from_size_align(128, 1).unwrap())
                .unwrap();
            regions.push(region);
        }

        for region in regions.drain(..) {
            unsafe {
                allocator.dealloc(region);
            }
        }

        assert_eq!(allocator.blocks.len(), 1);
    }

    fn find_overlapping_regions(regions: &[Region]) -> Option<(Region, Region)> {
        for (lhs_i, lhs) in regions.iter().enumerate() {
            for (rhs_i, rhs) in regions.iter().enumerate() {
                if lhs_i == rhs_i {
                    continue;
                }

                if lhs.start() < rhs.end() && rhs.start() < lhs.end() {
                    return Some((*lhs, *rhs));
                }
            }
        }

        None
    }
}
