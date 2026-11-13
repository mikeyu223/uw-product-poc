export type RiskBucket = "low" | "moderate" | "elevated" | "high";

export interface RiskDriver {
  code: string;
  label: string;
  contribution: number;
  /** Hand-tuned ceiling — show as contribution/max_points (e.g. 4/20). */
  max_points: number;
  detail?: string | null;
  family?: string;
}

export interface FamilyScore {
  family: string;
  label: string;
  points: number;
  max_points: number;
}

export interface ScoreBreakdown {
  gamma_regime: number;
  flip_proximity: number;
  wall_conflict: number;
  adverse_flow: number;
  structure: number;
  catalyst: number;
}

export interface Position {
  id: string;
  ticker: string;
  option_type: string;
  strike: number;
  expiry: string;
  quantity: number;
  side: string;
  underlying_price: number;
  delta: number;
  gamma: number;
  vega: number;
  position_id: string;
  score: number;
  score_max?: number;
  bucket: RiskBucket;
  drivers: RiskDriver[];
  breakdown?: ScoreBreakdown;
  family_scores?: FamilyScore[];
  methodology?: string;
  data_quality?: string;
  gex_snapshot?: {
    gamma_flip?: string | null;
    call_wall?: string | null;
    put_wall?: string | null;
    gamma_magnet?: string | null;
    net_gamma?: number | null;
    positive_gamma_regime?: boolean | null;
  };
}

export interface PortfolioSummary {
  position_count: number;
  avg_risk_score: number;
  bucket: RiskBucket;
  bucket_label: string;
  elevated_or_high: number;
  top_drivers: Array<{
    ticker?: string;
    label: string;
    contribution: number;
    detail?: string | null;
    family?: string;
  }>;
  updated_at: string;
  mode: string;
  methodology?: string;
  cache_hit?: boolean;
  cache_age_secs?: number;
  cache_ttl_secs?: number;
}

export interface ProbeResponse {
  ticker: string;
  info: {
    symbol: string;
    full_name?: string | null;
  };
  gex: {
    call_wall?: string | null;
    put_wall?: string | null;
    gamma_magnet?: string | null;
    gamma_flip?: string | null;
    nearby_flips?: string[] | null;
    source?: string | null;
    time?: string | null;
  };
  latest_greek?: {
    call_gamma?: string | null;
    put_gamma?: string | null;
    date?: string | null;
  } | null;
  recent_alerts: Array<{
    type?: string;
    option_type?: string;
    expiry?: string | null;
    strike?: string | null;
    total_premium?: string | null;
    alert_rule?: string | null;
    created_at?: string | null;
  }>;
}

export interface Methodology {
  audience: string;
  families?: Array<{
    family: string;
    label: string;
    max_points: number;
    why: string;
  }>;
  mix?: Array<{
    family: string;
    max_points: number;
    title: string;
    why: string;
  }>;
  note: string;
}
