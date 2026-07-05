// Formatting helpers shared across components.

/** Live reset countdown recomputed from resetAt (server's resetText goes stale). */
export function formatReset(resetAt?: number, nowMs: number = Date.now()): string {
  if (!resetAt) return "—";
  const ms = resetAt - nowMs;
  if (ms <= 0) return "已重置";
  const mins = Math.ceil(ms / 60000);
  if (mins < 60) return `${mins} 分钟后`;
  const hours = Math.floor(mins / 60);
  const rem = mins % 60;
  if (hours < 48) return rem ? `${hours} 小时 ${rem} 分后` : `${hours} 小时后`;
  const days = Math.floor(hours / 24);
  const rh = hours % 24;
  return rh ? `${days} 天 ${rh} 小时后` : `${days} 天后`;
}

export function formatNumber(n?: number): string {
  if (n == null) return "—";
  return n.toLocaleString("en-US", { maximumFractionDigits: 0 });
}

export function formatPercent(p?: number): string {
  if (p == null) return "—";
  return `${p.toFixed(p < 10 ? 1 : 0)}%`;
}

export function relativeTime(iso?: string): string {
  if (!iso) return "—";
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return "—";
  const diff = Date.now() - then;
  if (diff < 0) return "刚刚";
  if (diff < 60000) return "刚刚";
  const mins = Math.floor(diff / 60000);
  if (mins < 60) return `${mins} 分钟前`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours} 小时前`;
  return `${Math.floor(hours / 24)} 天前`;
}

/** Short Chinese label for a period (5h / session / weekly / monthly). */
export function periodLabel(label: string): string {
  switch (label) {
    case "5h": return "5 小时";
    case "session": return "会话";
    case "weekly": return "本周";
    case "monthly": return "本月";
    default: return label;
  }
}

export function planTitle(product: string): string {
  switch (product) {
    case "agent-plan": return "Agent Plan";
    case "coding-plan": return "Coding Plan";
    case "agent-plan-team": return "Agent Plan (团队)";
    case "coding-plan-team": return "Coding Plan (团队)";
    default: return product;
  }
}
