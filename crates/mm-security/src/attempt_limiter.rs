use std::collections::HashMap;
use std::fmt;
use std::net::IpAddr;
use std::sync::Mutex;

pub const DEFAULT_PAIRING_ATTEMPT_LIMIT: u8 = 5;
pub const DEFAULT_PAIRING_ATTEMPT_WINDOW_MS: u64 = 60_000;
pub const DEFAULT_PAIRING_COOLDOWN_MS: u64 = 30_000;
pub const DEFAULT_MAX_TRACKED_PAIRING_SOURCES: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairingAttemptPolicy {
    max_attempts: u8,
    window_ms: u64,
    cooldown_ms: u64,
    max_sources: usize,
}

impl PairingAttemptPolicy {
    pub fn new(
        max_attempts: u8,
        window_ms: u64,
        cooldown_ms: u64,
        max_sources: usize,
    ) -> Result<Self, PairingAttemptPolicyError> {
        if max_attempts == 0 {
            return Err(PairingAttemptPolicyError::ZeroAttemptLimit);
        }
        if window_ms == 0 {
            return Err(PairingAttemptPolicyError::ZeroWindow);
        }
        if cooldown_ms == 0 {
            return Err(PairingAttemptPolicyError::ZeroCooldown);
        }
        if max_sources == 0 {
            return Err(PairingAttemptPolicyError::ZeroSourceCapacity);
        }
        Ok(Self {
            max_attempts,
            window_ms,
            cooldown_ms,
            max_sources,
        })
    }
}

impl Default for PairingAttemptPolicy {
    fn default() -> Self {
        Self {
            max_attempts: DEFAULT_PAIRING_ATTEMPT_LIMIT,
            window_ms: DEFAULT_PAIRING_ATTEMPT_WINDOW_MS,
            cooldown_ms: DEFAULT_PAIRING_COOLDOWN_MS,
            max_sources: DEFAULT_MAX_TRACKED_PAIRING_SOURCES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingAttemptPolicyError {
    ZeroAttemptLimit,
    ZeroWindow,
    ZeroCooldown,
    ZeroSourceCapacity,
}

impl fmt::Display for PairingAttemptPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for PairingAttemptPolicyError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingAttemptDecision {
    Allowed,
    CoolingDown { retry_after_ms: u64 },
    CapacityLimited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingAttemptLimiterError {
    LockPoisoned,
}

impl fmt::Display for PairingAttemptLimiterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for PairingAttemptLimiterError {}

#[derive(Debug)]
struct AttemptEntry {
    window_started_at_ms: u64,
    attempts: u8,
    cooldown_deadline_ms: Option<u64>,
    cooldown_indefinite: bool,
}

impl AttemptEntry {
    fn new(now_ms: u64) -> Self {
        Self {
            window_started_at_ms: now_ms,
            attempts: 0,
            cooldown_deadline_ms: None,
            cooldown_indefinite: false,
        }
    }

    fn decision(&self, now_ms: u64) -> PairingAttemptDecision {
        if self.cooldown_indefinite {
            return PairingAttemptDecision::CoolingDown {
                retry_after_ms: u64::MAX,
            };
        }
        if let Some(deadline_ms) = self.cooldown_deadline_ms {
            return PairingAttemptDecision::CoolingDown {
                retry_after_ms: deadline_ms.saturating_sub(now_ms),
            };
        }
        PairingAttemptDecision::Allowed
    }

    fn is_expired(&self, now_ms: u64, policy: PairingAttemptPolicy) -> bool {
        if self.cooldown_indefinite {
            return false;
        }
        if let Some(deadline_ms) = self.cooldown_deadline_ms {
            return now_ms >= deadline_ms;
        }
        now_ms.saturating_sub(self.window_started_at_ms) >= policy.window_ms
    }
}

#[derive(Debug, Default)]
struct LimiterState {
    attempts_by_source: HashMap<IpAddr, AttemptEntry>,
}

/// Bounded in-memory pairing attempt limiter keyed by the remote source IP.
///
/// The caller supplies monotonic milliseconds from one clock domain and must call
/// `begin_attempt` before proof verification. A successful pairing removes that source's
/// attempt history. When all source slots are occupied, unknown addresses are denied until
/// expired entries are pruned.
pub struct PairingAttemptLimiter {
    policy: PairingAttemptPolicy,
    state: Mutex<LimiterState>,
}

impl Default for PairingAttemptLimiter {
    fn default() -> Self {
        Self::new(PairingAttemptPolicy::default())
    }
}

impl PairingAttemptLimiter {
    pub fn new(policy: PairingAttemptPolicy) -> Self {
        Self {
            policy,
            state: Mutex::new(LimiterState::default()),
        }
    }

    /// Atomically reserves one attempt before any proof verification work starts.
    pub fn begin_attempt(
        &self,
        source: IpAddr,
        now_monotonic_ms: u64,
    ) -> Result<PairingAttemptDecision, PairingAttemptLimiterError> {
        let source = normalize_source(source);
        let mut state = self
            .state
            .lock()
            .map_err(|_| PairingAttemptLimiterError::LockPoisoned)?;
        prune_expired(&mut state, now_monotonic_ms, self.policy);
        let current = match state.attempts_by_source.get(&source) {
            Some(entry) => entry.decision(now_monotonic_ms),
            None if state.attempts_by_source.len() >= self.policy.max_sources => {
                PairingAttemptDecision::CapacityLimited
            }
            None => PairingAttemptDecision::Allowed,
        };
        if current != PairingAttemptDecision::Allowed {
            return Ok(current);
        }

        let entry = state
            .attempts_by_source
            .entry(source)
            .or_insert_with(|| AttemptEntry::new(now_monotonic_ms));
        entry.attempts = entry.attempts.saturating_add(1);
        if entry.attempts < self.policy.max_attempts {
            return Ok(PairingAttemptDecision::Allowed);
        }

        match now_monotonic_ms.checked_add(self.policy.cooldown_ms) {
            Some(deadline_ms) => {
                entry.cooldown_deadline_ms = Some(deadline_ms);
                Ok(PairingAttemptDecision::Allowed)
            }
            None => {
                entry.cooldown_indefinite = true;
                Ok(PairingAttemptDecision::Allowed)
            }
        }
    }

    /// Clears this source's attempt history after a successful pairing.
    pub fn record_success(&self, source: IpAddr) -> Result<(), PairingAttemptLimiterError> {
        let source = normalize_source(source);
        self.state
            .lock()
            .map_err(|_| PairingAttemptLimiterError::LockPoisoned)?
            .attempts_by_source
            .remove(&source);
        Ok(())
    }
}

fn normalize_source(source: IpAddr) -> IpAddr {
    match source {
        IpAddr::V6(address) => address
            .to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(address)),
        IpAddr::V4(address) => IpAddr::V4(address),
    }
}

fn prune_expired(state: &mut LimiterState, now_ms: u64, policy: PairingAttemptPolicy) {
    state
        .attempts_by_source
        .retain(|_, entry| !entry.is_expired(now_ms, policy));
}
