// Mirrors the Rust structs in src-tauri/src/{ark_usage,state}.rs (camelCase).

export interface Viewer {
  userName: string;
  accountId: string;
  tenant?: string;
  region?: string;
}

export interface Period {
  label: string;
  used?: number;
  total?: number;
  percent?: number;
  remainingPercent?: number;
  resetAt?: number; // epoch ms
  resetText?: string;
}

export interface Plan {
  product: string;
  edition: string;
  tier?: string;
  periods: Period[];
}

export interface PlanUsage {
  viewer: Viewer;
  plans: Plan[];
  fetchedAt: string;
  authExpired: boolean;
  notInstalled: boolean;
  error?: string;
}

export interface ModelUsage {
  model: string;
  amount: number;
  tokens?: number;
  records: number;
}

// Mirrors src-tauri/src/ark_billing.rs (camelCase). Monthly pay-as-you-go
// (beyond-plan) billing rolled up per model.
export interface BillingUsage {
  billPeriod: string;
  totalAmount: number;
  totalRecords: number;
  byModel: ModelUsage[];
  truncated: boolean;
  fetchedAt: string;
  authExpired: boolean;
  notInstalled: boolean;
  error?: string;
}

export interface Settings {
  refreshIntervalSecs: number;
  trayPlans: string[];
  trayPeriod: string;
  thresholdWarn: number;
  thresholdCritical: number;
  autostart: boolean;
}
