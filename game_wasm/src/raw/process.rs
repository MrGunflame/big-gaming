use game_macros::guest_only;

#[guest_only]
pub fn abort() -> !;
