import type { Viewer } from "../lib/types";
import { relativeTime } from "../lib/format";

interface Props {
  viewer: Viewer | null;
  fetchedAt: string;
  refreshing: boolean;
  onRefresh: () => void;
  onOpenSettings: () => void;
}

export function HeaderBar({ viewer, fetchedAt, refreshing, onRefresh, onOpenSettings }: Props) {
  return (
    <div className="header">
      <div className="header-id">
        <span className="header-name">{viewer?.userName || "—"}</span>
        {viewer?.region && <span className="header-acct">{viewer.region}</span>}
      </div>
      <div className="header-actions">
        <span className="header-time" title={fetchedAt}>
          {relativeTime(fetchedAt)}
        </span>
        <button
          className="icon-btn"
          onClick={onRefresh}
          disabled={refreshing}
          title="刷新 (⌘R)"
          aria-label="刷新"
        >
          <svg
            className={refreshing ? "spin" : ""}
            viewBox="0 0 24 24"
            width="16"
            height="16"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <polyline points="23 4 23 10 17 10" />
            <polyline points="1 20 1 14 7 14" />
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
          </svg>
        </button>
        <button className="icon-btn" onClick={onOpenSettings} title="设置 (⌘,)" aria-label="设置">
          <svg
            viewBox="0 0 24 24"
            width="16"
            height="16"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <line x1="4" y1="21" x2="4" y2="14" />
            <line x1="4" y1="10" x2="4" y2="3" />
            <line x1="12" y1="21" x2="12" y2="12" />
            <line x1="12" y1="8" x2="12" y2="3" />
            <line x1="20" y1="21" x2="20" y2="16" />
            <line x1="20" y1="12" x2="20" y2="3" />
            <line x1="1" y1="14" x2="7" y2="14" />
            <line x1="9" y1="8" x2="15" y2="8" />
            <line x1="17" y1="16" x2="23" y2="16" />
          </svg>
        </button>
      </div>
    </div>
  );
}
