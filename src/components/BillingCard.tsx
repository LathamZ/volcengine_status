import type { ReactNode } from "react";
import type { BillingUsage } from "../lib/types";
import { formatMoney, formatNumber } from "../lib/format";

interface Props {
  billing: BillingUsage | null;
}

export function BillingCard({ billing }: Props) {
  // Auth/install failure is already surfaced by the global AuthBanner /
  // InstallBanner (driven by plan usage); billing shares the same SSO session,
  // so stay quiet here rather than duplicating the prompt.
  if (billing?.authExpired || billing?.notInstalled) return null;

  let body: ReactNode;
  if (!billing) {
    body = <div className="billing-loading">加载中…</div>;
  } else if (billing.error) {
    body = <div className="error-banner">⚠ {billing.error}</div>;
  } else if (billing.totalAmount === 0 && billing.byModel.length === 0) {
    body = <div className="billing-empty">{billing.billPeriod} 暂无超额消费</div>;
  } else {
    body = (
      <>
        <div className="billing-total">
          <span className="billing-amount">{formatMoney(billing.totalAmount)}</span>
          <span className="billing-meta">
            {billing.billPeriod} · {billing.totalRecords} 条
            {billing.truncated ? " · 仅前 50 条" : ""}
          </span>
        </div>
        <div className="billing-models">
          {billing.byModel.map((m) => (
            <div className="billing-model" key={m.model}>
              <span className="billing-model-name" title={m.model}>
                {m.model}
              </span>
              <span className="billing-model-meta">
                {m.tokens != null && `${formatNumber(m.tokens)} 千 tokens · `}
                {m.records} 条
              </span>
              <span className="billing-model-amount">{formatMoney(m.amount)}</span>
            </div>
          ))}
        </div>
      </>
    );
  }

  return (
    <div className="card">
      <div className="card-head">
        <span className="card-title">本月超额消费</span>
        <span className="badge badge-muted">按量计费</span>
      </div>
      {body}
    </div>
  );
}
