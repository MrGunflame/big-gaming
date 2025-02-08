pub mod arena;
pub mod bimap;
pub mod linked_list;
pub mod lru;
pub mod scratch_buffer;
pub mod sparse_set;
pub mod string;
pub mod vec_map;

unsafe trait IsZst: Sized {
    const IS_ZST: bool;
}

unsafe impl<T> IsZst for T {
    const IS_ZST: bool = size_of::<Self>() == 0;
}
