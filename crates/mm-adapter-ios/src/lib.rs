#[cfg(any(test, target_os = "ios"))]
mod identity;

#[cfg(any(test, target_os = "ios"))]
mod keychain;

#[cfg(any(test, target_os = "ios"))]
pub use keychain::IosPairingIdentity;

#[cfg(any(test, target_os = "ios"))]
mod runtime;

#[cfg(test)]
mod keychain_test_callbacks {
    use std::sync::Mutex;

    static TEST_KEYCHAIN_SEED: Mutex<Option<[u8; 32]>> = Mutex::new(None);

    /// # Safety
    /// `output` must be writable for `capacity` bytes when non-null.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn mirrormesh_ios_keychain_read_seed(
        output: *mut u8,
        capacity: usize,
    ) -> i32 {
        if output.is_null() || capacity < 32 {
            return -2;
        }
        let Ok(seed_slot) = TEST_KEYCHAIN_SEED.lock() else {
            return -1;
        };
        let Some(seed) = seed_slot.as_ref() else {
            return 1;
        };
        // SAFETY: The caller promises 32 writable bytes and this adapter writes exactly 32.
        unsafe { std::ptr::copy_nonoverlapping(seed.as_ptr(), output, 32) };
        0
    }

    /// # Safety
    /// `seed` must be readable for `length` bytes when non-null.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn mirrormesh_ios_keychain_write_seed(
        seed: *const u8,
        length: usize,
    ) -> i32 {
        if seed.is_null() || length != 32 {
            return -2;
        }
        let Ok(mut seed_slot) = TEST_KEYCHAIN_SEED.lock() else {
            return -1;
        };
        let mut stored = [0_u8; 32];
        // SAFETY: The caller promises 32 readable bytes and this adapter reads exactly 32.
        unsafe { std::ptr::copy_nonoverlapping(seed, stored.as_mut_ptr(), 32) };
        *seed_slot = Some(stored);
        0
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn mirrormesh_ios_keychain_delete_seed() -> i32 {
        let Ok(mut seed_slot) = TEST_KEYCHAIN_SEED.lock() else {
            return -1;
        };
        if let Some(seed) = seed_slot.as_mut() {
            seed.fill(0);
        }
        *seed_slot = None;
        0
    }
}
