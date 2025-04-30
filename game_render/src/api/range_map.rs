use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::ops::{Add, BitAnd, BitOr, BitOrAssign, Bound, Range, RangeBounds};

use bitflags::Flags;
use smallvec::SmallVec;

/// A range map optimized for a small number of disjointed ranges.
#[derive(Clone, Debug)]
pub struct RangeMap<K, V> {
    bounds: Range<K>,
    ranges: SmallVec<[(Range<K>, V); 1]>,
}

impl<K, V> RangeMap<K, V>
where
    K: Key + Default,
    V: Copy + Eq,
{
    pub fn new(bounds: Range<K>) -> Self {
        Self {
            bounds,
            ranges: SmallVec::new(),
        }
    }

    pub fn from_full_range(range: Range<K>, value: V) -> Self {
        Self {
            bounds: range.clone(),
            ranges: SmallVec::from([(range, value)]),
        }
    }

    pub fn insert<T>(&mut self, range: T, value: V)
    where
        T: RangeBounds<K>,
    {
        let start = match range.start_bound() {
            Bound::Included(start) => *start,
            Bound::Excluded(start) => *start + K::from(1),
            Bound::Unbounded => self.bounds.start,
        };
        let end = match range.end_bound() {
            Bound::Included(end) => *end + K::from(1),
            Bound::Excluded(end) => *end,
            Bound::Unbounded => self.bounds.end,
        };

        self.ranges.push((start..end, value));
    }

    pub fn iter(&self) -> impl Iterator<Item = &(Range<K>, V)> {
        self.ranges.iter()
    }

    pub fn compact(&mut self)
    where
        V: Default + BitOr + BitAnd + BitOrAssign + Flags + Eq + BitOr<Output = V>,
        K: std::fmt::Debug,
        V: std::fmt::Debug,
    {
        if self.ranges.len() == 1 {
            return;
        }

        self.ranges
            .sort_unstable_by(|(lhs, _), (rhs, _)| lhs.start.cmp(&rhs.start));

        let mut new_ranges = SmallVec::<[(Range<K>, V); 1]>::new();

        let mut current_bits = V::empty();
        let mut active_ranges = BinaryHeap::<ActiveRange<K, V>>::new();
        let mut current_range = Range::default();
        for (range, bits) in &self.ranges {
            while let Some(active_range) = active_ranges.peek().copied() {
                if active_range.end.0 > range.start {
                    break;
                }

                debug_assert!(current_bits.contains(active_range.bits));

                if current_range.start < active_range.end.0 {
                    let r = current_range.start..active_range.end.0;
                    debug_assert!(!r.is_empty());
                    new_ranges.push((r, current_bits));

                    current_range.start = active_range.end.0;
                }

                // Reconstruct the bits from the set of all currently
                // active ranges.
                // FIXME: It would be more efficient to just remove
                // the bits from the current `active_range`, but this
                // does not work correctly if ranges overlap that contain
                // the same bit.
                active_ranges.pop();
                current_bits = V::default();
                for active_range in active_ranges.iter() {
                    current_bits.insert(active_range.bits);
                }
            }

            active_ranges.push(ActiveRange {
                end: Reverse(range.end),
                bits: *bits,
            });

            if current_bits | *bits != current_bits
                && !(current_range.start..range.start).is_empty()
            {
                let r = current_range.start..range.start;
                new_ranges.push((r, current_bits));
                current_range.start = range.start;
            }

            current_bits |= *bits;
            if range.end > current_range.end {
                current_range.end = range.end;
            }
        }

        while let Some(active_range) = active_ranges.pop() {
            let range = current_range.start..active_range.end.0;
            if !range.is_empty() {
                new_ranges.push((range, current_bits));
            }

            current_range.start = active_range.end.0;
            current_bits.remove(active_range.bits);
        }

        // Merge adjacent ranges with equal bits together.
        // FIXME: This could actually be done in the loop above.
        let mut index = 0;
        while index < new_ranges.len() {
            let (Some((a_range, a_bits)), Some((b_range, b_bits))) =
                (new_ranges.get(index), new_ranges.get(index + 1))
            else {
                break;
            };

            debug_assert!(b_range.start >= a_range.end);

            if a_range.end == b_range.start && a_bits == b_bits {
                new_ranges[index].0.end = b_range.end;
                new_ranges.remove(index + 1);
            } else {
                index += 1;
            }
        }

        self.ranges = new_ranges;
    }
}

pub trait Key: Copy + Eq + Ord + Add<Output = Self> + From<u8> {}

impl<T> Key for T where T: Copy + Eq + Ord + Add<Output = Self> + From<u8> {}

#[derive(Copy, Clone, Debug)]
struct ActiveRange<K, V> {
    end: Reverse<K>,
    bits: V,
}

impl<K, V> PartialEq for ActiveRange<K, V>
where
    K: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.end.eq(&other.end)
    }
}

impl<K, V> Eq for ActiveRange<K, V> where K: Eq {}

impl<K, V> PartialOrd for ActiveRange<K, V>
where
    K: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.end.partial_cmp(&other.end)
    }
}

impl<K, V> Ord for ActiveRange<K, V>
where
    K: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.end.cmp(&other.end)
    }
}

#[cfg(test)]
mod tests {
    use bitflags::bitflags;

    use super::RangeMap;

    bitflags! {
        #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
        struct TestFlags: u8 {
            const A = 1;
            const B = 2;
            const C = 4;
            const D = 8;
        }
    }

    #[test]
    fn range_map_compact_single() {
        let mut map = RangeMap::<u32, TestFlags>::new(0..100);
        map.insert(0..100, TestFlags::A);
        map.compact();
        assert_eq!(map.ranges.as_slice(), &[(0..100, TestFlags::A)]);
    }

    #[test]
    fn range_map_compact_overlapping() {
        let mut map = RangeMap::<u32, TestFlags>::new(0..100);
        map.insert(30..80, TestFlags::A);
        map.insert(10..70, TestFlags::B);
        map.insert(0..90, TestFlags::C);
        map.insert(40..100, TestFlags::D);

        map.compact();
        assert_eq!(
            map.ranges.as_slice(),
            &[
                (0..10, TestFlags::C),
                (10..30, TestFlags::B | TestFlags::C),
                (30..40, TestFlags::A | TestFlags::B | TestFlags::C),
                (
                    40..70,
                    TestFlags::A | TestFlags::B | TestFlags::C | TestFlags::D
                ),
                (70..80, TestFlags::A | TestFlags::C | TestFlags::D),
                (80..90, TestFlags::C | TestFlags::D),
                (90..100, TestFlags::D)
            ]
        );
    }

    #[test]
    fn range_map_compact_range_ends_in_middle() {
        let mut map = RangeMap::<u32, TestFlags>::new(0..100);
        map.insert(0..100, TestFlags::A);
        map.insert(0..10, TestFlags::B);
        map.insert(0..20, TestFlags::C);
        map.insert(20..30, TestFlags::B);
        map.insert(60..70, TestFlags::D);

        map.compact();
        assert_eq!(
            map.ranges.as_slice(),
            &[
                (0..10, TestFlags::A | TestFlags::B | TestFlags::C),
                (10..20, TestFlags::A | TestFlags::C),
                (20..30, TestFlags::A | TestFlags::B),
                (30..60, TestFlags::A),
                (60..70, TestFlags::A | TestFlags::D),
                (70..100, TestFlags::A),
            ]
        );
    }

    #[test]
    fn range_map_compact_merge() {
        let mut map = RangeMap::<u32, TestFlags>::new(0..100);
        map.insert(0..20, TestFlags::A);
        map.insert(20..80, TestFlags::A);
        map.insert(80..100, TestFlags::A);

        map.compact();
        assert_eq!(map.ranges.as_slice(), &[(0..100, TestFlags::A)]);
    }

    #[test]
    fn range_map_compact_overlapping_duplicates() {
        let mut map = RangeMap::<u32, TestFlags>::new(0..100);
        map.insert(0..20, TestFlags::A);
        map.insert(10..20, TestFlags::A | TestFlags::B);
        map.insert(20..40, TestFlags::C);
        map.insert(20..60, TestFlags::A);
        map.insert(30..40, TestFlags::A);
        map.insert(50..60, TestFlags::A);

        map.compact();
        assert_eq!(
            map.ranges.as_slice(),
            &[
                (0..10, TestFlags::A),
                (10..20, TestFlags::A | TestFlags::B),
                (20..40, TestFlags::A | TestFlags::C),
                (40..60, TestFlags::A),
            ]
        );
    }
}
