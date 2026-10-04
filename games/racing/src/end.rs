//! How a race ends, in racing's own terms, and the small adapter onto the engine's
//! `EndReason` (racing.md "How a race ends").
//!
//! A finished race reports `EndReason::Finished` (M3a, #56) in its `Outcome`; the tick
//! cap reports `EndReason::TickLimit`. Racing code reads [`RaceEnd`] (from
//! [`crate::RacingRules::race_end`]), which also says which of the two is a
//! truncation.

use engine::generic::EndReason;

/// The engine reason a finished race reports.
pub const FINISHED_REASON: EndReason = EndReason::Finished;

/// Why a race ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RaceEnd {
    /// Every racing car has finished, or the finish window after the winner closed
    /// (*terminated*).
    Finished,
    /// The tick cap was reached, even inside the finish window (*truncated*).
    TickLimit,
}

impl RaceEnd {
    /// The engine reason recorded in `Outcome` (and so in replays).
    pub fn reason(self) -> EndReason {
        match self {
            RaceEnd::Finished => FINISHED_REASON,
            RaceEnd::TickLimit => EndReason::TickLimit,
        }
    }

    /// Read a racing outcome's reason back.
    pub fn from_reason(reason: EndReason) -> Option<Self> {
        match reason {
            EndReason::TickLimit => Some(RaceEnd::TickLimit),
            EndReason::Finished => Some(RaceEnd::Finished),
            _ => None,
        }
    }

    /// Gymnasium/PettingZoo: true for the tick cap, false for a real finish.
    pub fn is_truncated(self) -> bool {
        self == RaceEnd::TickLimit
    }
}
