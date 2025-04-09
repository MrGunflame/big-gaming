use std::alloc::Layout;
use std::num::NonZeroUsize;

use game_common::utils::vec_ext::VecExt;
use game_tracing::trace_span;
use slab::Slab;

use super::{Allocator, Region};

#[derive(Clone, Debug)]
pub struct BuddyAllocator {
    blocks: Slab<Block>,
    root: usize,
    stack: Vec<usize>,
}

impl BuddyAllocator {
    pub fn new(region: Region) -> Self {
        // TODO: Handle regions not starting at 0.
        assert_eq!(region.offset, 0);
        assert!(region.size.is_power_of_two());

        let mut blocks = Slab::new();
        let root = blocks.insert(Block {
            offset: 0,
            size: region.size,
            state: State::Free,
            parent: None,
        });

        Self {
            blocks,
            root,
            // To walk down the tree we start at the root node and in
            // every step:
            // 1. Pop a node from the stack
            // 2. Push the left node
            // 3. Push the right node if needed
            // This means while traversing the tree we will have at most
            // "depth" right nodes in the stack, i.e. log2(size).
            // One extra slot is needed for the left node, right before it is
            // popped again.
            // This is the worst-case size of the stack and it never needs to grow.
            // Note that is stack is relatively small (e.g. 520 bytes for a 2**64 byte region).
            stack: Vec::with_capacity((region.size.ilog2() + 1) as usize),
        }
    }
}

impl Allocator for BuddyAllocator {
    fn alloc(&mut self, layout: Layout) -> Option<Region> {
        let _span = trace_span!("BuddyAllocator::alloc").entered();

        // We can only allocate in power-of-two blocks so we must always
        // round up.
        let size = layout.size().next_power_of_two();
        debug_assert_ne!(size, 0);

        let align = layout.align();
        debug_assert!(align.is_power_of_two());

        self.stack.clear();
        unsafe {
            self.stack.push_unchecked(self.root);
        }

        while let Some(index) = self.stack.pop() {
            let block = unsafe { self.blocks.get_unchecked_mut(index) };

            if size > block.size {
                continue;
            }

            match block.state {
                State::Free => (),
                State::Used => continue,
                State::Split { left, right } => {
                    // Walk down the left subtree first.
                    // This will keep smaller allocations on the left side.

                    // The left block will be at the same offset as the parent
                    // block.
                    // The right block will be at `offset + size / 2`. This means
                    // we can only allocate in the right block if the alignment
                    // is small enough.
                    if (block.offset + block.size / 2) % align == 0 {
                        unsafe {
                            self.stack.push_unchecked(right.get());
                        }
                    }

                    unsafe {
                        self.stack.push_unchecked(left.get());
                    }

                    continue;
                }
            }

            // Block is as small as possible for the allocation.
            // Note that this will always match at some point
            // since both `block.size` and `size` are power-of-twos.
            if block.size == size {
                block.state = State::Used;

                debug_assert!(block.offset % align == 0);

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

            // Reserve capacity for both blocks then insert both
            // without doing bounds checks.
            // This means the branch to grow the slab only exists
            // once. The compiler is unable to elide this otherwise.
            self.blocks.reserve(2);
            let left = unsafe { self.blocks.insert_unchecked(left) };
            let right = unsafe { self.blocks.insert_unchecked(right) };

            // Mark the block as split.
            let block = unsafe { self.blocks.get_unchecked_mut(index) };
            block.state = State::Split {
                // Safety: The children can never be the root node which has index 0.
                left: unsafe { NonZeroUsize::new_unchecked(left) },
                right: unsafe { NonZeroUsize::new_unchecked(right) },
            };

            // Since the parent block that we just split is bigger
            // than the requested size, either of the split blocks
            // will be big enough for `size`.
            // This means we only need to walk down the left side.
            debug_assert!(block.size >= size);

            unsafe {
                self.stack.push_unchecked(left);
            }
        }

        None
    }

    unsafe fn dealloc(&mut self, region: Region) {
        let _span = trace_span!("BuddyAllocator::dealloc").entered();

        let mut index = self.root;
        let mut block = unsafe { self.blocks.get_unchecked_mut(self.root) };
        while block.offset != region.offset || block.size != region.size {
            let (left, right) = match block.state {
                State::Split { left, right } => (left, right),
                // Since the caller guarantees that the region was returned
                // from `alloc` we can be sure that the block was always split.
                _ => unsafe { core::hint::unreachable_unchecked() },
            };

            let mid = block.offset + block.size / 2;
            index = if region.offset < mid {
                left.get()
            } else {
                right.get()
            };
            block = unsafe { self.blocks.get_unchecked_mut(index) };
        }

        // Mark the deallocated block as free.
        debug_assert!(matches!(block.state, State::Used));
        block.state = State::Free;

        // Attempt to merge the freed block.
        let mut block = unsafe { self.blocks.get_unchecked(index) };
        while let Some(parent_index) = block.parent {
            let parent = unsafe { self.blocks.get_unchecked(parent_index) };

            let (left, right) = match parent.state {
                State::Split { left, right } => (left, right),
                // If the children block has a parent of `Some(..)` the
                // parent block must have been split.
                _ => unsafe { core::hint::unreachable_unchecked() },
            };

            let other = if index == left.get() { right } else { left };
            debug_assert_ne!(index, other.get());

            // If our buddy is not free we cannot merge any further.
            if unsafe { !self.blocks.get_unchecked(other.get()).state.is_free() } {
                break;
            }

            // Merge left and right back into parent.
            unsafe {
                self.blocks.remove_unchecked(left.get());
                self.blocks.remove_unchecked(right.get());
            }

            let parent = unsafe { self.blocks.get_unchecked_mut(parent_index) };
            parent.state = State::Free;

            index = parent_index;
            block = unsafe { self.blocks.get_unchecked(parent_index) };
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
    Split {
        // The children of the block that was split.
        // Note that the root block has the index 0 and
        // no node can every have the root as a children.
        // This means we can use the `NonZero` variant.
        left: NonZeroUsize,
        right: NonZeroUsize,
    },
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

    #[test]
    fn buddy_allocator_big_alignment() {
        let mut allocator = BuddyAllocator::new(Region::new(0, 2048));

        for _ in 0..4 {
            // Do a small allocation to cause fragmentation.
            allocator
                .alloc(Layout::from_size_align(1, 1).unwrap())
                .unwrap();

            allocator
                .alloc(Layout::from_size_align(128, 256).unwrap())
                .unwrap();
        }
    }
}
