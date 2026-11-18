export interface FuturesContract {
  name: string;
  product?: string | null;
  expiry?: string | null;
  tick_size?: string | null;
  volume?: number | null;
  trades?: number | null;
}

export interface TapePrint {
  trade_id?: number | null;
  t: string;
  side: string;
  size: number;
  price: number;
  price_display: string;
  is_block: boolean;
  is_large: boolean;
  is_spread: boolean;
  signed_size: number;
  cvd: number;
  nbbo_bid?: number | null;
  nbbo_ask?: number | null;
}

export interface CvdPoint {
  t: string;
  price: number;
  signed_size: number;
  buy_size: number;
  sell_size: number;
  cvd: number;
  net_delta: number;
}

export interface PriceLevelDelta {
  price: number;
  buy_size: number;
  sell_size: number;
  net_delta: number;
  total_size: number;
  intensity: number;
}

export interface OrderFlowStats {
  buy_volume: number;
  sell_volume: number;
  buy_pct: number;
  sell_pct: number;
  large_prints: number;
  block_prints: number;
  max_print_size: number;
  aggression: string;
}

export interface CvdSummary {
  trade_count: number;
  buy_volume: number;
  sell_volume: number;
  net_delta: number;
  cvd_high: number;
  cvd_low: number;
  cvd_last: number;
  last_price?: number | null;
  first_ts?: string | null;
  last_ts?: string | null;
  tick_size: number;
  bias: string;
  narrative: string;
}

export interface FuturesStats {
  open_interest?: number | null;
  session_high?: string | null;
  session_low?: string | null;
  session_volume?: number | null;
  settlement?: string | null;
  settlement_date?: string | null;
}

export interface ProfileLevel {
  price: number;
  volume: number;
  buy_volume: number;
  sell_volume: number;
  in_value_area: boolean;
  is_poc: boolean;
}

export interface VolumeProfile {
  tick_size: number;
  poc: number;
  vah: number;
  val: number;
  total_volume: number;
  value_area_pct: number;
  levels: ProfileLevel[];
  methodology: string;
}

export interface NamedLevel {
  name: string;
  price: number;
  kind: string;
}

export interface SessionLevels {
  globex_high?: number | null;
  globex_low?: number | null;
  globex_mid?: number | null;
  settlement?: number | null;
  settlement_date?: string | null;
  rth_open?: number | null;
  rth_high?: number | null;
  rth_low?: number | null;
  rth_last?: number | null;
  opening_range_high?: number | null;
  opening_range_low?: number | null;
  in_rth: boolean;
  bias_vs_globex: string;
  levels: NamedLevel[];
  narrative: string;
  methodology: string;
}

export interface FuturesGexLevel {
  name: string;
  price: number;
  kind: string;
}

export interface GexExpiryColumn {
  expiry: string;
  dte?: number | null;
  label: string;
  net_gex: number;
}

export interface GexHeatRow {
  strike: number;
  values: number[];
  intensities: number[];
  sum_net: number;
}

export interface GexHeatmap {
  proxy_ticker: string;
  spot?: number | null;
  move_high?: number | null;
  move_low?: number | null;
  expiries: GexExpiryColumn[];
  rows: GexHeatRow[];
  max_abs: number;
  strike_min: number;
  strike_max: number;
  methodology: string;
}

export interface FuturesGex {
  product: string;
  proxy_ticker: string;
  futures_last?: number | null;
  proxy_spot?: number | null;
  zero_gamma_flip?: number | null;
  call_wall?: number | null;
  put_wall?: number | null;
  gamma_magnet?: number | null;
  net_gamma?: number | null;
  regime: string;
  regime_label: string;
  distance_to_flip?: number | null;
  distance_to_flip_pct?: number | null;
  position_vs_flip: string;
  levels: FuturesGexLevel[];
  heatmap?: GexHeatmap | null;
  narrative: string;
  methodology: string;
}

export interface CvdProfile {
  contract: string;
  product: string;
  summary: CvdSummary;
  order_flow: OrderFlowStats;
  tape: TapePrint[];
  series: CvdPoint[];
  levels: PriceLevelDelta[];
  volume_profile: VolumeProfile;
  sessions: SessionLevels;
  futures_gex: FuturesGex;
  stats?: FuturesStats | null;
  methodology: string;
}

export interface OiRow {
  product: string;
  front: string;
  back?: string | null;
  front_volume: number;
  back_volume: number;
  open_interest?: number | null;
  session_volume?: number | null;
  session_high?: number | null;
  session_low?: number | null;
  settlement?: number | null;
  turnover?: number | null;
  regime: string;
  note: string;
}

export interface OiMatrixReport {
  rows: OiRow[];
  narrative: string;
  methodology: string;
}

export interface SweepMarker {
  t: string;
  premium: number;
  side: string;
  mins_before_entry: number;
  label: string;
}

export interface FlowContext {
  alignment: string;
  badge: string;
  net_call_prem: number;
  net_put_prem: number;
  nearest_sweep_prem?: number | null;
  nearest_sweep_mins?: number | null;
  nearest_sweep_side?: string | null;
  gex_regime: string;
  gex_label: string;
  net_gamma?: number | null;
  major_sweeps: SweepMarker[];
}

export interface TradeEntry {
  id: string;
  account: string;
  symbol: string;
  asset_class: string;
  side: string;
  qty: number;
  entry_price: number;
  exit_price: number;
  entry_at: string;
  exit_at: string;
  commissions: number;
  pnl: number;
  setup: string;
  notes: string;
  emotion: string;
  tags: string[];
  dte_at_entry?: number | null;
  iv_rank_at_entry?: number | null;
  contract_strategy: string;
  plan?: string | null;
  stop_loss?: number | null;
  take_profit?: number | null;
  risk_amount?: number | null;
  planned_rr?: number | null;
  followed_rules?: boolean | null;
  lesson?: string | null;
  flow?: FlowContext | null;
}

export interface PerformanceSummary {
  trade_count: number;
  wins: number;
  losses: number;
  win_rate: number;
  total_pnl: number;
  avg_win: number;
  avg_loss: number;
  profit_factor: number;
  expectancy: number;
  payoff_ratio: number;
  sharpe_approx: number;
  sortino_approx: number;
}

export interface AccountRow {
  account: string;
  trades: number;
  pnl: number;
  win_rate: number;
}

export interface SetupRow {
  setup: string;
  trades: number;
  wins: number;
  win_rate: number;
  total_pnl: number;
  avg_r: number;
  expectancy: number;
}

export interface HeatCell {
  label: string;
  trades: number;
  pnl: number;
  win_rate: number;
}

export interface JournalFilters {
  account?: string | null;
  setup?: string | null;
  tod?: string | null;
  dow?: string | null;
  strategy?: string | null;
  dte_bucket?: string | null;
}

export interface JournalAnalytics {
  summary: PerformanceSummary;
  accounts: AccountRow[];
  by_setup: SetupRow[];
  by_strategy: SetupRow[];
  time_of_day: HeatCell[];
  day_of_week: HeatCell[];
  recent: TradeEntry[];
  active_filters: JournalFilters;
}

export interface ChartCandle {
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
}

export interface ChartMarker {
  time: number;
  position: string;
  color: string;
  shape: string;
  text: string;
  kind: string;
}

export interface TradeChart {
  trade_id: string;
  symbol: string;
  candles: ChartCandle[];
  markers: ChartMarker[];
  entry_time: number;
  exit_time: number;
  source: string;
}

export interface TermPoint {
  days: number;
  iv: number;
  implied_move_pct?: number | null;
  percentile?: number | null;
}

export interface IvHistoryPoint {
  date: string;
  iv: number;
  iv_rank?: number | null;
  close?: number | null;
}

export interface EarningsName {
  ticker: string;
  name?: string | null;
  sector?: string | null;
  earnings_date?: string | null;
  earnings_confirmed: boolean;
  days_to_earnings?: number | null;
  spot?: number | null;
  iv_rank?: number | null;
  front_iv?: number | null;
  iv_30d?: number | null;
  crush_ratio?: number | null;
  implied_move_pct?: number | null;
  crush_pressure: number;
  crush_label: string;
  term: TermPoint[];
  iv_history: IvHistoryPoint[];
  takeaway: string;
}

export interface EarningsIvReport {
  as_of: string;
  names: EarningsName[];
  narrative: string;
  methodology: string;
}

export interface SweepAlert {
  id: string;
  ticker: string;
  option_type: string;
  strike: string;
  expiry: string;
  premium: number;
  size: number;
  side_bias: string;
  ask_prem: number;
  bid_prem: number;
  has_sweep: boolean;
  alert_rule: string;
  created_at: string;
  iv?: number | null;
  dte: number;
}

export interface SweepCluster {
  key: string;
  ticker: string;
  bucket_start: string;
  alert_count: number;
  sweep_count: number;
  total_premium: number;
  net_ask_prem: number;
  net_bid_prem: number;
  dominant_side: string;
  call_premium: number;
  put_premium: number;
  cluster_score: number;
  label: string;
  alerts: SweepAlert[];
}

export interface SweepClusterReport {
  as_of: string;
  session_date: string;
  total_alerts_scanned: number;
  zero_dte_alerts: number;
  sweep_alerts: number;
  clusters: SweepCluster[];
  feed: SweepAlert[];
  narrative: string;
  methodology: string;
}
