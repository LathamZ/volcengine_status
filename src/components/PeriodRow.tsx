import type { Period } from "../lib/types";
import { formatNumber, formatPercent, formatReset, periodLabel } from "../lib/format";

interface Props {
  period: Period;
  warn: number;
  critical: number;
  now: number; // forces re-render every tick so the countdown stays live
}

function level(percent: number | undefined, warn: number, critical: number): "ok" | "warn" | "critical" | "none" {
  if (percent == null) return "none";
  if (percent >= critical) return "critical";
  if (percent >= warn) return "warn";
  return "ok";
}

export function PeriodRow({ period, warn, critical, now }: Props) {
  const pct = period.percent;
  const lvl = level(pct, warn, critical);
  const width = pct != null ? `${Math.min(100, Math.max(0, pct))}%` : "0%";
  const hasAbsolute = period.used != null && period.total != null;

  return (
    <div className="period-row">
      <div className="period-top">
        <span className="period-name">{periodLabel(period.label)}</span>
        {hasAbsolute && (
          <span className="period-meta">
            {formatNumber(period.used)} / {formatNumber(period.total)}
          </span>
        )}
        <span className={`period-pct pct-${lvl}`}>{formatPercent(pct)}</span>
        <span className="period-reset" title="重置时间">
          {formatReset(period.resetAt, now)}
        </span>
      </div>
      <div className="bar">
        <div className={`bar-fill bar-${lvl}`} style={{ width }} />
      </div>
    </div>
  );
}
