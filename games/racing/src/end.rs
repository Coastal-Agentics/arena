//! How a race ends, in racing's own terms, and the small adapter onto the engine's
//! `EndReason` (racing.md "How a race ends").
//!
//! `EndReason::Finished` arrives with Shockwave's M3a. Until it is on `main`, a
//! finished race reports [`FINISHED_REASON`] (`EndReason::LastStanding`, the closest
//! existing *terminated* reason) in its `Outcome`, and racing code reads
//! [`RaceEnd`] (from [`crate::RacingRules::race_end`]) instead of matching on the
//! engine enum. Swapping in the real variant is a one-line change here.

use engine::generic::EndReason;

/// The engine reason a finished race reports until M3a's `EndReason::Finished` lands.
pub const FINISHED_REASON: EndReason = EndReason::LastStanding;

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
            r if r == FINISHED_REASON => Some(RaceEnd::Finished),
            _ => None,
        }
    }

    /// Gymnasium/PettingZoo: true for the tick cap, false for a real finish.
    pub fn is_truncated(self) -> bool {
        self == RaceEnd::TickLimit
    }
}
