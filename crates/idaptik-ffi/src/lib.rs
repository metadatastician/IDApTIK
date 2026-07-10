//! C-ABI surface over [`idaptik_core`].
//!
//! This is the stable boundary that non-Rust code binds to: the **Zig** FFI
//! bridge for APIs, and the **Idris2** models that specify the ABI contracts
//! (ADR-0001). Keep this layer thin — it owns only pointer/lifetime handling and
//! delegates every decision to `idaptik-core`.
//!
//! Regenerate the C header with cbindgen:
//! ```text
//! cbindgen --crate idaptik-ffi --output include/idaptik.h
//! ```
//!
//! Edition 2024 requires the `unsafe(...)` wrapper on `#[no_mangle]`.

use idaptik_core::{Network, demo_network};

/// Opaque owner of a [`Network`]. Consumers hold this as an opaque pointer and
/// only touch it through the functions below.
pub struct NetworkHandle(Network);

/// Build the demonstration network. The caller owns the returned pointer and
/// must release it with [`idap_network_free`].
#[unsafe(no_mangle)]
pub extern "C" fn idap_demo_network() -> *mut NetworkHandle {
    Box::into_raw(Box::new(NetworkHandle(demo_network())))
}

/// Free a network previously returned by this library. Passing null is a no-op.
///
/// # Safety
/// `ptr` must be a pointer returned by this library and not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn idap_network_free(ptr: *mut NetworkHandle) {
    if !ptr.is_null() {
        drop(unsafe { Box::from_raw(ptr) });
    }
}

/// Number of devices in the network. Returns 0 for a null pointer.
///
/// # Safety
/// `ptr` must be null or a valid pointer from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn idap_network_device_count(ptr: *const NetworkHandle) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { &*ptr }.0.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_network_roundtrips_through_the_abi() {
        let handle = idap_demo_network();
        assert!(!handle.is_null());
        // SAFETY: handle came straight from idap_demo_network above.
        let count = unsafe { idap_network_device_count(handle) };
        assert_eq!(count, 6);
        unsafe { idap_network_free(handle) };
    }

    #[test]
    fn null_is_handled() {
        assert_eq!(unsafe { idap_network_device_count(std::ptr::null()) }, 0);
        unsafe { idap_network_free(std::ptr::null_mut()) }; // no-op, must not crash
    }
}
