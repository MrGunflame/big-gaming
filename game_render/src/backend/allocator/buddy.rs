use std::alloc::Layout;
use std::collections::VecDeque;

use game_tracing::trace_span;
use slab::Slab;

use super::Region;

#[derive(Clone, Debug)]
pub struct BuddyAllocator {
    region: Region,
    blocks: Slab<Block>,
}

impl BuddyAllocator {
    pub fn new(region: Region) -> Self {
        assert!(region.size.is_power_of_two());

        let mut blocks = Slab::new();
        blocks.insert(Block {
            size: region.size,
            offset: 0,
            is_free: true,
            children: None,
            parent: None,
        });

        Self { region, blocks }
    }

    pub fn alloc(&mut self, layout: Layout) -> Option<Region> {
        let _span = trace_span!("BuddyAllocator::alloc").entered();

        let size = layout.size().next_power_of_two();

        let mut queue = VecDeque::new();
        queue.push_back(0);

        while let Some(index) = queue.pop_front() {
            let block = &mut self.blocks[index];

            if block.is_free && block.size == size {
                debug_assert!(block.children.is_none());

                block.is_free = false;
                return Some(Region::new(self.region.offset + block.offset, block.size));
            }

            if block.size < block.size {
                continue;
            }

            let children = match block.children {
                Some((left, right)) => [left, right],
                None => {
                    // Cannot divide this block further.
                    // Or the block is already occupied.
                    if block.size == 1 || !block.is_free {
                        continue;
                    }

                    // Split the block.
                    block.is_free = false;

                    let left = Block {
                        offset: block.offset,
                        size: block.size / 2,
                        is_free: true,
                        children: None,
                        parent: Some(index),
                    };
                    let right = Block {
                        offset: block.offset + block.size / 2,
                        size: block.size / 2,
                        is_free: true,
                        children: None,
                        parent: Some(index),
                    };

                    let left = self.blocks.insert(left);
                    let right = self.blocks.insert(right);

                    let block = &mut self.blocks[index];
                    block.children = Some((left, right));

                    [left, right]
                }
            };

            queue.extend(children);
        }

        None
    }

    pub unsafe fn dealloc(&mut self, region: Region) {
        let _span = trace_span!("BuddyAllocator::dealloc").entered();

        let (mut index, _) = self
            .blocks
            .iter()
            .find(|(_, block)| block.offset == region.offset && block.size == region.size)
            .unwrap();

        let block = &mut self.blocks[index];

        debug_assert!(!block.is_free);
        debug_assert!(block.children.is_none());

        block.is_free = true;
        let mut block = &self.blocks[index];

        while let Some(parent_index) = block.parent {
            let parent = &self.blocks[parent_index];

            let (left, right) = parent.children.unwrap();
            debug_assert!(index == left || index == right);
            let other = if left == index { right } else { left };
            debug_assert_ne!(index, other);

            // If our buddy is not free we cannot merge further.
            if !self.blocks[other].is_free {
                break;
            }

            self.blocks.remove(left);
            self.blocks.remove(right);

            let parent = &mut self.blocks[parent_index];
            parent.children = None;
            parent.is_free = true;
            let parent = &self.blocks[parent_index];

            index = parent_index;
            block = parent;
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Block {
    size: usize,
    offset: usize,
    is_free: bool,
    children: Option<(usize, usize)>,
    parent: Option<usize>,
}

#[cfg(test)]
mod tests {
    use std::alloc::Layout;

    use crate::backend::allocator::Region;

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
