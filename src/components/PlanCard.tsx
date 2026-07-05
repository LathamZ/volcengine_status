import type { Plan } from "../lib/types";
import { planTitle } from "../lib/format";
import { PeriodRow } from "./PeriodRow";

interface Props {
  plan: Plan;
  warn: number;
  critical: number;
  now: number;
}

export function PlanCard({ plan, warn, critical, now }: Props) {
  return (
    <div className="card">
      <div className="card-head">
        <span className="card-title">{planTitle(plan.product)}</span>
        {plan.tier && <span className="badge">{plan.tier}</span>}
        {plan.edition && (
          <span className="badge badge-muted">{plan.edition}</span>
        )}
      </div>
      <div className="periods">
        {plan.periods.map((p) => (
          <PeriodRow key={p.label} period={p} warn={warn} critical={critical} now={now} />
        ))}
      </div>
    </div>
  );
}
