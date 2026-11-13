import type { RiskBucket } from "./types";

export function bucketLabel(bucket: RiskBucket): string {
  switch (bucket) {
    case "low":
      return "Low";
    case "moderate":
      return "Moderate";
    case "elevated":
      return "Elevated";
    case "high":
      return "High";
  }
}

export function bucketColor(bucket: RiskBucket): string {
  switch (bucket) {
    case "low":
      return "text-emerald-700";
    case "moderate":
      return "text-amber-700";
    case "elevated":
      return "text-orange-700";
    case "high":
      return "text-rose-700";
  }
}

export function bucketBg(bucket: RiskBucket): string {
  switch (bucket) {
    case "low":
      return "bg-emerald-50 border-emerald-200";
    case "moderate":
      return "bg-amber-50 border-amber-200";
    case "elevated":
      return "bg-orange-50 border-orange-200";
    case "high":
      return "bg-rose-50 border-rose-200";
  }
}

/** Hex fills for light UI. */
export function scoreBarHex(score: number): string {
  if (score < 25) return "#16a34a";
  if (score < 50) return "#d97706";
  if (score < 75) return "#ea580c";
  return "#e11d48";
}

export function formatCompactNumber(n: number): string {
  const abs = Math.abs(n);
  const sign = n < 0 ? "-" : "";
  if (abs >= 1_000_000) return `${sign}${(abs / 1_000_000).toFixed(2)}M`;
  if (abs >= 1_000) return `${sign}${(abs / 1_000).toFixed(1)}K`;
  return `${sign}${abs.toFixed(0)}`;
}
