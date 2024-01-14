use crate::events::Event;

pub fn register_event_handler<T>(f: fn(event: T))
where
    T: Event,
{
    let fn_ptr = f as *const fn();

    unsafe {
        crate::raw::register_event_handler(&T::ID, fn_ptr);
    }
}
