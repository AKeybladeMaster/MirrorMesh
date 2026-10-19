#[cfg(windows)]
mod identity;

#[cfg(windows)]
mod runtime;

#[cfg(windows)]
pub use identity::WindowsPairingIdentity;

#[cfg(windows)]
pub use runtime::initialize_pairing_runtime;

#[cfg(all(test, windows))]
mod tests;
