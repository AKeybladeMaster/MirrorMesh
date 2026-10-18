#[cfg(windows)]
mod identity;

#[cfg(windows)]
pub use identity::WindowsPairingIdentity;

#[cfg(all(test, windows))]
mod tests;
