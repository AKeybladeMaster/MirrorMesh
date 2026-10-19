#![cfg(any(test, target_os = "android"))]

mod identity;
mod runtime;

#[cfg(any(test, target_os = "android"))]
mod android_store;

#[cfg(target_os = "android")]
pub use android_store::AndroidPairingIdentity;

#[cfg(target_os = "android")]
pub use runtime::initialize_pairing_runtime;

#[cfg(test)]
mod tests;
