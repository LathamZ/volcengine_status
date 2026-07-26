import { useState } from "react";
import type { ReactNode } from "react";
import type { BillingUsage } from "../lib/types";
import { formatMoney, formatNumber } from "../lib/format";

interface Props {
  billing: BillingUsage | null;
}

export function BillingCard({ billing }: Props) {
  const [showAll, setShowAll] = useState(false);

  if (billing?.authExpired || billing?.notInstalled) return null;

  // Sorted by amount desc so the most expensive models surface first.
  const sorted = billing
    ? [...billing.byModel].sort((a, b) => b.amount - a.amount)
    : [];
  const top = sorted.slice(0, 3);
  const hasMore = sorted.length > 3;

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
          {top.map((m) => (
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
        {hasMore && (
          <button className="billing-more" onClick={() => setShowAll(true)}>
            查看全部 {sorted.length} 个
          </button>
        )}
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
      {showAll && billing && (
        <div className="settings-overlay" onClick={() => setShowAll(false)}>
          <div className="settings-panel" role="dialog" onClick={(e) => e.stopPropagation()}>
            <div className="settings-head">
              <strong>超额消费明细 · {billing.billPeriod}</strong>
              <button className="settings-close" onClick={() => setShowAll(false)} aria-label="关闭">×</button>
            </div>
            <div className="settings-body">
              <div className="billing-models">
                {sorted.map((m) => (
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
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
