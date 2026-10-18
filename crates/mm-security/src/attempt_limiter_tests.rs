use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use super::{
    PairingAttemptDecision as Decision, PairingAttemptLimiter, PairingAttemptPolicy,
    PairingAttemptPolicyError,
};

fn policy(
    max_attempts: u8,
    window_ms: u64,
    cooldown_ms: u64,
    max_sources: usize,
) -> PairingAttemptPolicy {
    PairingAttemptPolicy::new(max_attempts, window_ms, cooldown_ms, max_sources).unwrap()
}

fn source(last_octet: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(192, 168, 1, last_octet))
}

#[test]
fn blocks_after_the_attempt_limit_and_allows_a_new_window_after_cooldown() {
    let limiter = PairingAttemptLimiter::new(policy(3, 1_000, 250, 8));

    assert_eq!(
        limiter.begin_attempt(source(10), 100),
        Ok(Decision::Allowed)
    );
    assert_eq!(
        limiter.begin_attempt(source(10), 200),
        Ok(Decision::Allowed)
    );
    assert_eq!(
        limiter.begin_attempt(source(10), 300),
        Ok(Decision::Allowed)
    );
    assert_eq!(
        limiter.begin_attempt(source(10), 300),
        Ok(Decision::CoolingDown {
            retry_after_ms: 250
        })
    );
    assert_eq!(
        limiter.begin_attempt(source(10), 549),
        Ok(Decision::CoolingDown { retry_after_ms: 1 })
    );
    assert_eq!(
        limiter.begin_attempt(source(10), 550),
        Ok(Decision::Allowed)
    );
}

#[test]
fn twenty_attempt_admissions_from_one_address_are_throttled_between_batches() {
    let limiter = PairingAttemptLimiter::new(policy(5, 60_000, 30_000, 8));
    let mut now = 1_000;
    let mut cooling_down = 0;

    for _ in 0..20 {
        match limiter.begin_attempt(source(10), now).unwrap() {
            Decision::Allowed => {}
            Decision::CoolingDown { retry_after_ms } => {
                cooling_down += 1;
                now += retry_after_ms;
                assert_eq!(
                    limiter.begin_attempt(source(10), now),
                    Ok(Decision::Allowed)
                );
            }
            Decision::CapacityLimited => panic!("configured source capacity is sufficient"),
        }

        now += 1;
    }

    assert_eq!(cooling_down, 3);
    assert!(matches!(
        limiter.begin_attempt(source(10), now),
        Ok(Decision::CoolingDown { .. })
    ));
}

#[test]
fn concurrent_requests_cannot_exceed_the_per_source_admission_limit() {
    let limiter = Arc::new(PairingAttemptLimiter::new(policy(5, 60_000, 30_000, 8)));
    let workers: Vec<_> = (0..16)
        .map(|_| {
            let limiter = Arc::clone(&limiter);
            std::thread::spawn(move || limiter.begin_attempt(source(10), 1).unwrap())
        })
        .collect();

    let decisions: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();

    assert_eq!(
        decisions
            .iter()
            .filter(|decision| **decision == Decision::Allowed)
            .count(),
        5
    );
    assert_eq!(
        decisions
            .iter()
            .filter(|decision| matches!(decision, Decision::CoolingDown { .. }))
            .count(),
        11
    );
}

#[test]
fn addresses_have_independent_budgets_and_ipv4_mapped_ipv6_is_normalized() {
    let limiter = PairingAttemptLimiter::new(policy(2, 1_000, 200, 8));
    let ipv4 = source(10);
    let mapped = IpAddr::V6(Ipv4Addr::new(192, 168, 1, 10).to_ipv6_mapped());

    assert_eq!(limiter.begin_attempt(ipv4, 50), Ok(Decision::Allowed));
    assert_eq!(limiter.begin_attempt(mapped, 60), Ok(Decision::Allowed));
    assert_eq!(
        limiter.begin_attempt(ipv4, 61),
        Ok(Decision::CoolingDown {
            retry_after_ms: 199
        })
    );
    assert_eq!(limiter.begin_attempt(source(11), 61), Ok(Decision::Allowed));
}

#[test]
fn successful_pairing_clears_the_source_attempt_history() {
    let limiter = PairingAttemptLimiter::new(policy(2, 1_000, 200, 8));
    let peer = source(10);

    assert_eq!(limiter.begin_attempt(peer, 50), Ok(Decision::Allowed));
    assert_eq!(limiter.begin_attempt(peer, 60), Ok(Decision::Allowed));
    assert!(matches!(
        limiter.begin_attempt(peer, 61),
        Ok(Decision::CoolingDown { .. })
    ));
    limiter.record_success(peer).unwrap();
    assert_eq!(limiter.begin_attempt(peer, 70), Ok(Decision::Allowed));
    assert_eq!(limiter.begin_attempt(peer, 80), Ok(Decision::Allowed));
    assert!(matches!(
        limiter.begin_attempt(peer, 81),
        Ok(Decision::CoolingDown { .. })
    ));
}

#[test]
fn source_capacity_fails_closed_and_expired_entries_release_capacity() {
    let limiter = PairingAttemptLimiter::new(policy(3, 100, 50, 1));

    assert_eq!(limiter.begin_attempt(source(10), 10), Ok(Decision::Allowed));
    assert_eq!(
        limiter.begin_attempt(source(11), 20),
        Ok(Decision::CapacityLimited)
    );
    assert_eq!(
        limiter.begin_attempt(source(11), 110),
        Ok(Decision::Allowed)
    );
}

#[test]
fn policy_rejects_zero_limits_and_durations() {
    assert_eq!(
        PairingAttemptPolicy::new(0, 1_000, 1_000, 4),
        Err(PairingAttemptPolicyError::ZeroAttemptLimit)
    );
    assert_eq!(
        PairingAttemptPolicy::new(5, 0, 1_000, 4),
        Err(PairingAttemptPolicyError::ZeroWindow)
    );
    assert_eq!(
        PairingAttemptPolicy::new(5, 1_000, 0, 4),
        Err(PairingAttemptPolicyError::ZeroCooldown)
    );
    assert_eq!(
        PairingAttemptPolicy::new(5, 1_000, 1_000, 0),
        Err(PairingAttemptPolicyError::ZeroSourceCapacity)
    );
}

#[test]
fn cooldown_deadline_overflow_fails_closed() {
    let limiter = PairingAttemptLimiter::new(policy(1, 1, 5, 1));

    assert_eq!(
        limiter.begin_attempt(source(10), u64::MAX - 1),
        Ok(Decision::Allowed)
    );
    assert_eq!(
        limiter.begin_attempt(source(10), u64::MAX),
        Ok(Decision::CoolingDown {
            retry_after_ms: u64::MAX
        })
    );
}
