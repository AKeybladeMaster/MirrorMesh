#![cfg(any(test, target_os = "android"))]

mod identity;

#[cfg(any(test, target_os = "android"))]
mod android_store;

#[cfg(target_os = "android")]
pub use android_store::AndroidPairingIdentity;

#[cfg(test)]
mod tests;
