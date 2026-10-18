mod identity;

#[cfg(target_os = "linux")]
pub use identity::LinuxPairingIdentity;

#[cfg(test)]
mod tests;
