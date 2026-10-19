mod identity;

#[cfg(any(test, target_os = "linux"))]
mod runtime;

#[cfg(target_os = "linux")]
pub use identity::LinuxPairingIdentity;

#[cfg(target_os = "linux")]
pub use runtime::initialize_pairing_runtime;

#[cfg(test)]
mod tests;
