pub mod span;

#[cfg(feature = "tracy")]
mod allocator;
#[cfg(feature = "tracy")]
mod layer;

#[cfg(feature = "tracy")]
pub use allocator::ProfiledAllocator;
#[cfg(feature = "tracy")]
pub use layer::TracyLayer;
#[cfg(feature = "tracy")]
pub use tracy_client::Client;

#[cfg(feature = "tracy")]
#[cfg_attr(all(not(miri), not(test)), global_allocator)]
static GLOBAL: ProfiledAllocator<std::alloc::System> = ProfiledAllocator::new(std::alloc::System);
