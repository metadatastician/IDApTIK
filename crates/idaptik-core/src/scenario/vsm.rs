//! First VSM-shaped supervision model for NPC teams and adaptive difficulty.
//!
//! This is an experiment, not yet part of the Ghost Lobby simulation. It keeps
//! the useful VSM roles explicit and deterministic so the model can be tested
//! before it is connected to gameplay events.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Functional VSM roles. These are roles, not required processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VsmSystem {
    Operations,
    Coordination,
    Regulation,
    Audit,
    Intelligence,
    Policy,
}

/// An NPC that can receive a team order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operator {
    pub id: String,
    pub role: String,
    pub readiness: u8,
}

/// A recursive operational unit: a team is made from operators and may itself
/// be an operator at a higher faction/session level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorTeam {
    pub id: String,
    pub operators: Vec<Operator>,
    pub attention: u8,
}

/// An observation about a player's tactic. It is deliberately a hypothesis
/// input, not a claim that the player has a fixed identity or strategy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerTacticObservation {
    pub tactic_id: String,
    pub occurrence: u32,
    pub confidence: u8,
}

/// A bounded adaptation proposed by the intelligence function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DifficultyProposal {
    pub from: u8,
    pub to: u8,
    pub reason: String,
    pub tactic_id: String,
}

/// A bounded, provenance-bearing hypothesis. Confidence is represented as
/// basis points (`0..=1000`) rather than floating point so replay is exact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hypothesis {
    pub proposition: String,
    pub confidence: u16,
    pub supporting_events: Vec<String>,
    pub contrary_events: Vec<String>,
}

/// Evidence updates one or more competing explanations. The ledger never sees
/// hidden world truth; callers provide only evidence available to the observer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HypothesisEvidence {
    pub event_id: String,
    pub support: Vec<(String, u16)>,
    pub contrary: Vec<(String, u16)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HypothesisLedger {
    pub hypotheses: Vec<Hypothesis>,
}

impl HypothesisLedger {
    pub fn new(propositions: impl IntoIterator<Item = String>) -> Self {
        Self {
            hypotheses: propositions
                .into_iter()
                .map(|proposition| Hypothesis {
                    proposition,
                    confidence: 500,
                    supporting_events: Vec::new(),
                    contrary_events: Vec::new(),
                })
                .collect(),
        }
    }

    pub fn observe(&mut self, evidence: HypothesisEvidence) {
        for (proposition, amount) in evidence.support {
            if let Some(hypothesis) = self
                .hypotheses
                .iter_mut()
                .find(|candidate| candidate.proposition == proposition)
            {
                hypothesis.confidence = hypothesis.confidence.saturating_add(amount).min(1000);
                hypothesis.supporting_events.push(evidence.event_id.clone());
            }
        }
        for (proposition, amount) in evidence.contrary {
            if let Some(hypothesis) = self
                .hypotheses
                .iter_mut()
                .find(|candidate| candidate.proposition == proposition)
            {
                hypothesis.confidence = hypothesis.confidence.saturating_sub(amount);
                hypothesis.contrary_events.push(evidence.event_id.clone());
            }
        }
    }

    /// Ties resolve to declaration order, making the result stable across
    /// platforms and independent of map iteration order.
    pub fn most_likely(&self) -> Option<&Hypothesis> {
        self.hypotheses.iter().max_by_key(|hypothesis| hypothesis.confidence)
    }
}

/// Events make supervision and adaptation visible in a replayable trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum VsmEvent {
    PlayerTacticObserved {
        tactic_id: String,
        occurrence: u32,
        confidence: u8,
    },
    TeamAttentionAllocated {
        team_id: String,
        attention: u8,
    },
    DifficultyProposalGenerated(DifficultyProposal),
    DifficultyPolicyChecked {
        accepted: bool,
        target: u8,
    },
    AdaptiveInterventionApplied {
        from: u8,
        to: u8,
        tactic_id: String,
    },
    AuditRecorded {
        system: VsmSystem,
        message: String,
    },
}

/// Small deterministic director for a first adaptive-difficulty experiment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VsmDirector {
    pub current_difficulty: u8,
    pub maximum_difficulty: u8,
    pub observations: BTreeMap<String, u32>,
    pub trace: Vec<VsmEvent>,
}

impl VsmDirector {
    pub fn new(current_difficulty: u8, maximum_difficulty: u8) -> Self {
        Self {
            current_difficulty: current_difficulty.min(maximum_difficulty),
            maximum_difficulty,
            observations: BTreeMap::new(),
            trace: Vec::new(),
        }
    }

    /// Record an observation and produce a proposal once the tactic repeats.
    /// The threshold is intentionally simple and will move into a profile later.
    pub fn observe(&mut self, observation: PlayerTacticObservation) -> Option<DifficultyProposal> {
        let count = self
            .observations
            .entry(observation.tactic_id.clone())
            .or_insert(0);
        *count = (*count).max(observation.occurrence);
        self.trace.push(VsmEvent::PlayerTacticObserved {
            tactic_id: observation.tactic_id.clone(),
            occurrence: *count,
            confidence: observation.confidence.min(100),
        });

        if *count < 2 || observation.confidence < 60 {
            return None;
        }

        let target = self.current_difficulty.saturating_add(1).min(self.maximum_difficulty);
        Some(DifficultyProposal {
            from: self.current_difficulty,
            to: target,
            reason: "repeated tactic with sufficient confidence".into(),
            tactic_id: observation.tactic_id,
        })
    }

    /// Apply a proposal only inside the declared policy envelope.
    pub fn apply(&mut self, proposal: DifficultyProposal) -> bool {
        let accepted = proposal.from == self.current_difficulty
            && proposal.to >= proposal.from
            && proposal.to <= self.maximum_difficulty;
        self.trace.push(VsmEvent::DifficultyPolicyChecked {
            accepted,
            target: proposal.to,
        });
        if !accepted {
            self.trace.push(VsmEvent::AuditRecorded {
                system: VsmSystem::Audit,
                message: "difficulty proposal rejected by policy".into(),
            });
            return false;
        }
        self.current_difficulty = proposal.to;
        self.trace.push(VsmEvent::DifficultyProposalGenerated(proposal.clone()));
        self.trace.push(VsmEvent::AdaptiveInterventionApplied {
            from: proposal.from,
            to: proposal.to,
            tactic_id: proposal.tactic_id,
        });
        true
    }

    /// Allocate team attention as a bounded present-time regulation action.
    pub fn allocate_attention(&mut self, team: &mut OperatorTeam, attention: u8) {
        team.attention = attention.min(100);
        self.trace.push(VsmEvent::TeamAttentionAllocated {
            team_id: team.id.clone(),
            attention: team.attention,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_confident_tactic_proposes_bounded_intervention() {
        let mut director = VsmDirector::new(1, 2);
        assert!(director
            .observe(PlayerTacticObservation {
                tactic_id: "hidden-route".into(),
                occurrence: 1,
                confidence: 80,
            })
            .is_none());
        let proposal = director
            .observe(PlayerTacticObservation {
                tactic_id: "hidden-route".into(),
                occurrence: 2,
                confidence: 80,
            })
            .expect("repeat should produce a proposal");
        assert_eq!(proposal.to, 2);
        assert!(director.apply(proposal));
        assert_eq!(director.current_difficulty, 2);
    }

    #[test]
    fn policy_rejects_stale_or_over_limit_proposals() {
        let mut director = VsmDirector::new(1, 2);
        assert!(!director.apply(DifficultyProposal {
            from: 0,
            to: 2,
            reason: "stale".into(),
            tactic_id: "route".into(),
        }));
        assert!(!director.apply(DifficultyProposal {
            from: 1,
            to: 3,
            reason: "over limit".into(),
            tactic_id: "route".into(),
        }));
    }

    #[test]
    fn attention_is_bounded_and_traceable() {
        let mut director = VsmDirector::new(0, 1);
        let mut team = OperatorTeam {
            id: "patrol-a".into(),
            operators: vec![],
            attention: 0,
        };
        director.allocate_attention(&mut team, 120);
        assert_eq!(team.attention, 100);
        assert!(matches!(
            director.trace.last(),
            Some(VsmEvent::TeamAttentionAllocated { attention: 100, .. })
        ));
    }

    #[test]
    fn ledger_can_hold_a_rational_but_wrong_usb_hypothesis() {
        let mut ledger = HypothesisLedger::new([
            "player_wants_usb".into(),
            "player_wants_fridge_note".into(),
        ]);
        ledger.observe(HypothesisEvidence {
            event_id: "usb-interest-1".into(),
            support: vec![("player_wants_usb".into(), 180)],
            contrary: vec![],
        });
        ledger.observe(HypothesisEvidence {
            event_id: "usb-interest-2".into(),
            support: vec![("player_wants_usb".into(), 180)],
            contrary: vec![],
        });
        assert_eq!(ledger.most_likely().unwrap().proposition, "player_wants_usb");

        // The player actually wanted the note, but the observer has not seen
        // that fact. Later evidence can revise the belief without erasing the
        // original supporting events.
        ledger.observe(HypothesisEvidence {
            event_id: "fridge-note-found".into(),
            support: vec![("player_wants_fridge_note".into(), 420)],
            contrary: vec![("player_wants_usb".into(), 260)],
        });
        assert_eq!(ledger.most_likely().unwrap().proposition, "player_wants_fridge_note");
        assert_eq!(ledger.hypotheses[0].supporting_events.len(), 2);
        assert_eq!(ledger.hypotheses[0].contrary_events, vec!["fridge-note-found"]);
    }
}
