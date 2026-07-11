//! The after-action debrief — grade, score breakdown, tags and prose, all as
//! data. Mirrors the Exchange-House export shape (a `format`-tagged record).
//!
//! The extract/fail *scoring* lives on [`crate::scenario::sim::GhostLobbySim`]
//! (it needs the full run state); this module owns the debrief data types and
//! the two pure selectors: [`grade_for`] and [`debrief_text`].

use crate::scenario::common::{Grade, Outcome, Tone};
use crate::scenario::state::Stats;
use crate::scenario::tuning::GradeBands;
use serde::{Deserialize, Serialize};

/// The after-action export format tag.
pub const DEBRIEF_FORMAT: &str = "idaptik-ghost-lobby-after-action-v1";

/// A single debrief tag (a coloured takeaway line).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    pub text: String,
    pub tone: Tone,
}

impl Tag {
    /// Build a tag from a static string and tone.
    pub fn new(text: &str, tone: Tone) -> Self {
        Self {
            text: text.to_owned(),
            tone,
        }
    }
}

/// The signed contribution of each scoring term (audit trail for the score).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub base: f64,
    pub note: f64,
    pub misdirect: f64,
    pub boss: f64,
    pub camera: f64,
    pub isolation: f64,
    pub chute: f64,
    pub usb_trace: f64,
    pub rescue: f64,
    pub time_bonus: f64,
    pub alert_penalty: f64,
    pub failed_actions_penalty: f64,
    /// The raw sum before the difficulty multiplier.
    pub raw: f64,
    pub score_mult: f64,
    /// The final `max(0, js_round(raw * score_mult))`.
    pub final_score: u32,
}

/// The complete after-action debrief.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Debrief {
    pub format: String,
    pub success: bool,
    pub reason: Outcome,
    pub title: String,
    pub summary: String,
    pub grade: Grade,
    pub score: u32,
    pub tags: Vec<Tag>,
    pub breakdown: ScoreBreakdown,
    pub debrief_text: String,
    pub stats: Stats,
    pub time_s: f64,
    pub max_alert: f64,
}

/// Grade band selection (raw for fail, post-multiplier for success).
pub fn grade_for(score: u32, success: bool, bands: &GradeBands) -> Grade {
    let s = f64::from(score);
    if !success {
        return if s >= bands.fail_c {
            Grade::C
        } else {
            Grade::D
        };
    }
    if s >= bands.s {
        Grade::S
    } else if s >= bands.a {
        Grade::A
    } else if s >= bands.b {
        Grade::B
    } else if s >= bands.c {
        Grade::C
    } else {
        Grade::D
    }
}

/// Select the reflective debrief paragraph, matching the prototype's `debrief`.
///
/// `has_note` is whether the contact lead was secured; `misled` is whether Billy
/// ended believing the USB mattered (has it, or reported it).
pub fn debrief_text(reason: Outcome, has_note: bool, misled: bool) -> String {
    match reason {
        Outcome::Partition => "The infiltrator stayed outside the uplink's reliable envelope until the building separated the team physically. Support is a spatial resource; a remote partner cannot help everywhere at once.".to_owned(),
        Outcome::Caught => "Billy did not defeat a super-agent. He reached an exposed person after the support relationship had no remaining response. The clean revision is better timing, better hiding, or a deliberate decoy — not more aggression.".to_owned(),
        Outcome::Lockdown => "Repeated interventions and detections accumulated into a building-wide opinion. The hacker's actions worked locally, but their side effects became the dominant system. Use fewer, better-timed interventions.".to_owned(),
        Outcome::Extracted => {
            if has_note && misled {
                "The team leaves with the real intelligence while Billy preserves the wrong explanation. The outcome is clean not because nothing went wrong, but because the errors were directed.".to_owned()
            } else if has_note {
                "The strategic objective survived, but Billy's interpretation was not fully controlled. A cleaner run would separate acquisition from visible urgency and reserve the USB for a deliberate narrative intervention.".to_owned()
            } else {
                "The team survived, which preserves the campaign, but did not solve the information problem. Leaving is necessary; leaving with the right epistemic residue is better.".to_owned()
            }
        }
    }
}
