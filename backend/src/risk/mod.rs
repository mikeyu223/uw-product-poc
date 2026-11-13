//! Risk scoring & explanation engine for options traders.
//!
//! Composite 0–100 score from live Unusual Whales microstructure + position structure.
//! Every driver is mathematical proximity-to-harm with a hand-tuned max (flip = **20**).
//! UI shows each driver as `contribution / max_points` (e.g. 4/20).

mod catalog;
mod curves;
mod engine;
mod portfolio;

pub use catalog::{
    DriverSpec, FamilySpec, MAX_ADVERSE_FLOW, MAX_CATALYST, MAX_FLIP_PROXIMITY, MAX_GAMMA_REGIME,
    MAX_IV_REGIME, MAX_STRUCTURE, MAX_WALL_CONFLICT, TOTAL_MAX, driver_catalog, family_catalog,
};
pub use engine::{MarketContext, PositionInput, score_position};
pub use portfolio::{ScoredPosition, sample_book, score_book};

use catalog::family_max;
use serde::{Deserialize, Serialize};

/// Discrete risk buckets shown in the dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskBucket {
    Low,
    Moderate,
    Elevated,
    High,
}

impl RiskBucket {
    pub fn from_score(score: u8) -> Self {
        match score {
            0..=24 => Self::Low,
            25..=49 => Self::Moderate,
            50..=74 => Self::Elevated,
            _ => Self::High,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Moderate => "Moderate",
            Self::Elevated => "Elevated",
            Self::High => "High",
        }
    }
}

/// A single explanatory driver contributing to the score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskDriver {
    pub code: String,
    pub label: String,
    /// Points awarded now (mathematical curve output).
    pub contribution: f64,
    /// Hand-tuned ceiling for this driver (show as contribution/max_points).
    pub max_points: f64,
    pub detail: Option<String>,
    pub family: String,
}

impl RiskDriver {
    pub fn new(
        code: impl Into<String>,
        label: impl Into<String>,
        contribution: f64,
        max_points: f64,
        family: impl Into<String>,
        detail: Option<String>,
    ) -> Self {
        Self {
            code: code.into(),
            label: label.into(),
            contribution: (contribution * 10.0).round() / 10.0,
            max_points,
            detail,
            family: family.into(),
        }
    }
}

/// Per-family point contributions (always includes max for X/max UI).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScoreBreakdown {
    pub gamma_regime: f64,
    pub flip_proximity: f64,
    pub wall_conflict: f64,
    pub adverse_flow: f64,
    pub iv_regime: f64,
    pub structure: f64,
    pub catalyst: f64,
}

impl ScoreBreakdown {
    pub fn total(&self) -> f64 {
        self.gamma_regime
            + self.flip_proximity
            + self.wall_conflict
            + self.adverse_flow
            + self.iv_regime
            + self.structure
            + self.catalyst
    }

    /// Family ledger for UI: always all rows with points/max.
    pub fn family_scores(&self) -> Vec<FamilyScore> {
        vec![
            FamilyScore {
                family: "gamma_regime".into(),
                label: "Dealer GEX regime".into(),
                points: round1(self.gamma_regime),
                max_points: family_max("gamma_regime"),
            },
            FamilyScore {
                family: "flip_proximity".into(),
                label: "Zero-gamma flip proximity".into(),
                points: round1(self.flip_proximity),
                max_points: family_max("flip_proximity"),
            },
            FamilyScore {
                family: "wall_conflict".into(),
                label: "Call/put wall conflict".into(),
                points: round1(self.wall_conflict),
                max_points: family_max("wall_conflict"),
            },
            FamilyScore {
                family: "adverse_flow".into(),
                label: "Adverse options flow".into(),
                points: round1(self.adverse_flow),
                max_points: family_max("adverse_flow"),
            },
            FamilyScore {
                family: "iv_regime".into(),
                label: "IV / vega regime".into(),
                points: round1(self.iv_regime),
                max_points: family_max("iv_regime"),
            },
            FamilyScore {
                family: "structure".into(),
                label: "Structure (DTE / moneyness)".into(),
                points: round1(self.structure),
                max_points: family_max("structure"),
            },
            FamilyScore {
                family: "catalyst".into(),
                label: "Earnings / catalyst".into(),
                points: round1(self.catalyst),
                max_points: family_max("catalyst"),
            },
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyScore {
    pub family: String,
    pub label: String,
    pub points: f64,
    pub max_points: f64,
}

/// Scored view of one options position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionRisk {
    pub position_id: String,
    pub score: u8,
    pub score_max: u8,
    pub bucket: RiskBucket,
    /// Active drivers only (contribution > 0), each with max_points for X/max display.
    pub drivers: Vec<RiskDriver>,
    pub breakdown: ScoreBreakdown,
    /// Always-complete family ledger (includes 0/max rows).
    pub family_scores: Vec<FamilyScore>,
    pub methodology: String,
    pub data_quality: String,
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}
