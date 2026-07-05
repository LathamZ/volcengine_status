import type { Settings } from "./types";

export const DEFAULT_SETTINGS: Settings = {
  refreshIntervalSecs: 300,
  trayPlans: ["agent-plan", "coding-plan"],
  trayPeriod: "monthly",
  thresholdWarn: 70,
  thresholdCritical: 90,
  autostart: false,
};

export const PERIOD_OPTIONS = [
  { value: "short", label: "短周期 (5h / session)" },
  { value: "weekly", label: "周 (weekly)" },
  { value: "monthly", label: "月 (monthly)" },
];

export const PLAN_OPTIONS = [
  { value: "agent-plan", label: "Agent Plan", prefix: "A" },
  { value: "coding-plan", label: "Coding Plan", prefix: "C" },
];

export const INTERVAL_OPTIONS = [
  { value: 60, label: "1 分钟" },
  { value: 300, label: "5 分钟" },
  { value: 600, label: "10 分钟" },
  { value: 1800, label: "30 分钟" },
];
