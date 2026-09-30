//! Multi-channel sensor voting architectures and degradation policies.
//!
//! Standards Reference: IEC 61511-1 / IEC 61508-2 architecture modeling.
//! Note: Virtual simulation implementation. Does not represent SIL certified hardware.

use nebudcs_model::tag::TagQuality;
use serde::{Deserialize, Serialize};

/// Direction of trip condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TripDirection {
    /// Trip initiates when PV >= threshold (High / High-High).
    #[default]
    High,
    /// Trip initiates when PV <= threshold (Low / Low-Low).
    Low,
}

/// Configurable multi-channel voting architectures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum VotingArchitecture {
    /// 1oo1: Single sensor channel (simplex).
    OneOutOfOne,
    /// 1oo2: 1-out-of-2 voting (dual redundant, safety-biased).
    OneOutOfTwo,
    /// 2oo2: 2-out-of-2 voting (dual redundant, availability-biased).
    TwoOutOfTwo,
    /// 2oo3: 2-out-of-3 voting (triple modular redundant - TMR).
    #[default]
    TwoOutOfThree,
    /// 2oo4: 2-out-of-4 voting (quad redundant).
    TwoOutOfFour,
}

impl VotingArchitecture {
    /// Nominal required channels for this architecture.
    pub fn required_channel_count(&self) -> usize {
        match self {
            Self::OneOutOfOne => 1,
            Self::OneOutOfTwo => 2,
            Self::TwoOutOfTwo => 2,
            Self::TwoOutOfThree => 3,
            Self::TwoOutOfFour => 4,
        }
    }
}

/// Degradation policy when one or more redundant sensor channels fail (`TagQuality::Bad`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DegradationPolicy {
    /// Degrade 2oo3 -> 1oo2 on first fault (conservative / fail-safe).
    #[default]
    DegradeTo1oo2,
    /// Degrade 2oo3 -> 2oo2 on first fault (availability-biased).
    DegradeTo2oo2,
    /// Degrade 1oo2 -> 1oo1 on first fault.
    DegradeTo1oo1,
    /// Immediate trip upon any diagnosed channel fault (strict fail-safe).
    TripOnFault,
    /// Freeze last valid voting state until fault resolves.
    FreezeState,
}

/// Individual sensor channel telemetry fed to a voting solver.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorChannel {
    /// Channel or transmitter identifier (e.g. "PT-101A").
    pub id: String,
    /// Bound process tag or HAL address.
    pub tag: String,
    /// Current measured engineering value.
    pub value: f64,
    /// Measurement quality status.
    pub quality: TagQuality,
    /// Optional engineering unit (e.g. "bar", "degC").
    pub unit: Option<String>,
    /// Diagnostic fault status (e.g. out of range, open circuit, frozen).
    pub faulted: bool,
}

impl SensorChannel {
    /// Creates a new healthy sensor channel.
    pub fn new(id: impl Into<String>, tag: impl Into<String>, value: f64) -> Self {
        Self {
            id: id.into(),
            tag: tag.into(),
            value,
            quality: TagQuality::Good,
            unit: None,
            faulted: false,
        }
    }

    /// Sets the channel quality.
    pub fn with_quality(mut self, quality: TagQuality) -> Self {
        self.quality = quality;
        if quality == TagQuality::Bad {
            self.faulted = true;
        }
        self
    }

    /// Sets the engineering unit.
    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// True if the channel is healthy and usable for safety decisions.
    pub fn is_healthy(&self) -> bool {
        !self.faulted && self.quality == TagQuality::Good
    }
}

/// Detailed result of a safety voting evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SifVotingResult {
    /// Whether the voted safety trip condition is active (demands trip).
    pub trip_demanded: bool,
    /// Total number of configured channels evaluated.
    pub total_channels: usize,
    /// Number of healthy sensor channels available.
    pub healthy_channels: usize,
    /// Number of faulted / bad quality channels detected.
    pub faulted_channels: usize,
    /// Number of healthy channels that exceeded the trip setpoint.
    pub voting_votes: usize,
    /// Whether voting is operating in a degraded mode due to channel faults.
    pub is_degraded: bool,
    /// Active effective voting scheme (e.g. "2oo3", "1oo2 (degraded)").
    pub effective_scheme: String,
    /// Diagnostic summary message.
    pub message: String,
}

/// Evaluates multi-channel sensor voting against threshold and degradation policies.
pub fn evaluate_voting(
    architecture: VotingArchitecture,
    policy: DegradationPolicy,
    direction: TripDirection,
    threshold: f64,
    channels: &[SensorChannel],
) -> SifVotingResult {
    let total = channels.len();
    let mut healthy_count = 0;
    let mut votes = 0;

    for ch in channels {
        if ch.is_healthy() {
            healthy_count += 1;
            let exceeds = match direction {
                TripDirection::High => ch.value >= threshold,
                TripDirection::Low => ch.value <= threshold,
            };
            if exceeds {
                votes += 1;
            }
        }
    }

    let fault_count = total.saturating_sub(healthy_count);
    let _is_degraded = fault_count > 0;

    match architecture {
        VotingArchitecture::OneOutOfOne => {
            if fault_count > 0 {
                let trip = policy == DegradationPolicy::TripOnFault;
                SifVotingResult {
                    trip_demanded: trip,
                    total_channels: total,
                    healthy_channels: healthy_count,
                    faulted_channels: fault_count,
                    voting_votes: votes,
                    is_degraded: true,
                    effective_scheme: "1oo1 (Faulted)".to_string(),
                    message: format!(
                        "Channel faulted: {}",
                        if trip {
                            "Fail-safe trip asserted"
                        } else {
                            "Trip suppressed"
                        }
                    ),
                }
            } else {
                SifVotingResult {
                    trip_demanded: votes >= 1,
                    total_channels: total,
                    healthy_channels: healthy_count,
                    faulted_channels: 0,
                    voting_votes: votes,
                    is_degraded: false,
                    effective_scheme: "1oo1".to_string(),
                    message: format!("1oo1 evaluation: {} votes / 1 required", votes),
                }
            }
        }
        VotingArchitecture::OneOutOfTwo => {
            match fault_count {
                0 => SifVotingResult {
                    trip_demanded: votes >= 1,
                    total_channels: total,
                    healthy_channels: healthy_count,
                    faulted_channels: 0,
                    voting_votes: votes,
                    is_degraded: false,
                    effective_scheme: "1oo2".to_string(),
                    message: format!("1oo2 nominal evaluation: {} votes", votes),
                },
                1 => {
                    if policy == DegradationPolicy::TripOnFault {
                        SifVotingResult {
                            trip_demanded: true,
                            total_channels: total,
                            healthy_channels: healthy_count,
                            faulted_channels: 1,
                            voting_votes: votes,
                            is_degraded: true,
                            effective_scheme: "1oo2 (Trip on Fault)".to_string(),
                            message:
                                "1 channel faulted: TripOnFault policy triggered fail-safe trip"
                                    .to_string(),
                        }
                    } else {
                        // Degrades to 1oo1 on the single surviving healthy channel
                        SifVotingResult {
                            trip_demanded: votes >= 1,
                            total_channels: total,
                            healthy_channels: healthy_count,
                            faulted_channels: 1,
                            voting_votes: votes,
                            is_degraded: true,
                            effective_scheme: "1oo2 -> 1oo1".to_string(),
                            message: format!(
                                "1oo2 degraded to 1oo1: {} vote from surviving channel",
                                votes
                            ),
                        }
                    }
                }
                _ => {
                    // Both channels faulted -> Unsafe condition, fail-safe trip
                    SifVotingResult {
                        trip_demanded: true,
                        total_channels: total,
                        healthy_channels: 0,
                        faulted_channels: fault_count,
                        voting_votes: 0,
                        is_degraded: true,
                        effective_scheme: "1oo2 (All Faulted)".to_string(),
                        message: "All 1oo2 channels faulted: fail-safe trip asserted".to_string(),
                    }
                }
            }
        }
        VotingArchitecture::TwoOutOfTwo => {
            match fault_count {
                0 => SifVotingResult {
                    trip_demanded: votes >= 2,
                    total_channels: total,
                    healthy_channels: healthy_count,
                    faulted_channels: 0,
                    voting_votes: votes,
                    is_degraded: false,
                    effective_scheme: "2oo2".to_string(),
                    message: format!("2oo2 nominal evaluation: {} votes / 2 required", votes),
                },
                1 => {
                    let trip = match policy {
                        DegradationPolicy::TripOnFault => true,
                        DegradationPolicy::DegradeTo1oo1 => votes >= 1,
                        _ => false, // 2oo2 cannot satisfy 2 votes with only 1 healthy sensor
                    };
                    SifVotingResult {
                        trip_demanded: trip,
                        total_channels: total,
                        healthy_channels: healthy_count,
                        faulted_channels: 1,
                        voting_votes: votes,
                        is_degraded: true,
                        effective_scheme: "2oo2 (Degraded)".to_string(),
                        message: format!("2oo2 with 1 fault: policy {:?} result: {}", policy, trip),
                    }
                }
                _ => SifVotingResult {
                    trip_demanded: true,
                    total_channels: total,
                    healthy_channels: 0,
                    faulted_channels: fault_count,
                    voting_votes: 0,
                    is_degraded: true,
                    effective_scheme: "2oo2 (All Faulted)".to_string(),
                    message: "All channels faulted in 2oo2: fail-safe trip asserted".to_string(),
                },
            }
        }
        VotingArchitecture::TwoOutOfThree => {
            match fault_count {
                0 => SifVotingResult {
                    trip_demanded: votes >= 2,
                    total_channels: total,
                    healthy_channels: 3,
                    faulted_channels: 0,
                    voting_votes: votes,
                    is_degraded: false,
                    effective_scheme: "2oo3".to_string(),
                    message: format!("2oo3 nominal evaluation: {} votes / 2 required", votes),
                },
                1 => {
                    let (trip, scheme) = match policy {
                        DegradationPolicy::DegradeTo1oo2 => (votes >= 1, "2oo3 -> 1oo2"),
                        DegradationPolicy::DegradeTo2oo2 => (votes >= 2, "2oo3 -> 2oo2"),
                        DegradationPolicy::TripOnFault => (true, "2oo3 -> TripOnFault"),
                        DegradationPolicy::DegradeTo1oo1 => (votes >= 1, "2oo3 -> 1oo1"),
                        DegradationPolicy::FreezeState => (false, "2oo3 -> Freeze"),
                    };
                    SifVotingResult {
                        trip_demanded: trip,
                        total_channels: total,
                        healthy_channels: 2,
                        faulted_channels: 1,
                        voting_votes: votes,
                        is_degraded: true,
                        effective_scheme: scheme.to_string(),
                        message: format!(
                            "1 channel faulted: degraded to {} (votes: {})",
                            scheme, votes
                        ),
                    }
                }
                _ => {
                    // 2 or 3 channels faulted: Loss of voting majority -> fail-safe trip
                    SifVotingResult {
                        trip_demanded: true,
                        total_channels: total,
                        healthy_channels: healthy_count,
                        faulted_channels: fault_count,
                        voting_votes: votes,
                        is_degraded: true,
                        effective_scheme: "2oo3 (Multiple Faults)".to_string(),
                        message: format!(
                            "{} of 3 channels faulted: fail-safe trip asserted",
                            fault_count
                        ),
                    }
                }
            }
        }
        VotingArchitecture::TwoOutOfFour => {
            match fault_count {
                0 => SifVotingResult {
                    trip_demanded: votes >= 2,
                    total_channels: total,
                    healthy_channels: 4,
                    faulted_channels: 0,
                    voting_votes: votes,
                    is_degraded: false,
                    effective_scheme: "2oo4".to_string(),
                    message: format!("2oo4 nominal evaluation: {} votes / 2 required", votes),
                },
                1 => {
                    // Degrades to 2oo3 among the 3 healthy channels
                    SifVotingResult {
                        trip_demanded: votes >= 2,
                        total_channels: total,
                        healthy_channels: 3,
                        faulted_channels: 1,
                        voting_votes: votes,
                        is_degraded: true,
                        effective_scheme: "2oo4 -> 2oo3".to_string(),
                        message: format!("1 fault in 2oo4: degraded to 2oo3 (votes: {})", votes),
                    }
                }
                2 => {
                    // Degrades to 1oo2 or 2oo2 based on policy
                    let trip = match policy {
                        DegradationPolicy::DegradeTo1oo2 => votes >= 1,
                        DegradationPolicy::DegradeTo2oo2 => votes >= 2,
                        _ => votes >= 2,
                    };
                    SifVotingResult {
                        trip_demanded: trip,
                        total_channels: total,
                        healthy_channels: 2,
                        faulted_channels: 2,
                        voting_votes: votes,
                        is_degraded: true,
                        effective_scheme: "2oo4 -> 1oo2/2oo2".to_string(),
                        message: format!(
                            "2 faults in 2oo4: degraded evaluation (votes: {})",
                            votes
                        ),
                    }
                }
                _ => SifVotingResult {
                    trip_demanded: true,
                    total_channels: total,
                    healthy_channels: healthy_count,
                    faulted_channels: fault_count,
                    voting_votes: votes,
                    is_degraded: true,
                    effective_scheme: "2oo4 (Severe Fault)".to_string(),
                    message: "3 or 4 channels faulted: fail-safe trip asserted".to_string(),
                },
            }
        }
    }
}
