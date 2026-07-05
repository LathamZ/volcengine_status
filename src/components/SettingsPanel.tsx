import type { Settings } from "../lib/types";
import { INTERVAL_OPTIONS, PERIOD_OPTIONS, PLAN_OPTIONS } from "../lib/settings";

interface Props {
  open: boolean;
  settings: Settings;
  onChange: (s: Settings) => void;
  onClose: () => void;
}

export function SettingsPanel({ open, settings, onChange, onClose }: Props) {
  if (!open) return null;

  const togglePlan = (value: string) => {
    const has = settings.trayPlans.includes(value);
    const next = has
      ? settings.trayPlans.filter((p) => p !== value)
      : [...settings.trayPlans, value];
    onChange({ ...settings, trayPlans: next });
  };

  return (
    <div className="settings-overlay" onClick={onClose}>
      <div className="settings-panel" role="dialog" onClick={(e) => e.stopPropagation()}>
        <div className="settings-head">
          <strong>设置</strong>
          <button className="settings-close" onClick={onClose} aria-label="关闭">×</button>
        </div>
        <div className="settings-body">
          <label className="setting">
            <span className="setting-label">刷新间隔</span>
            <select
              value={settings.refreshIntervalSecs}
              onChange={(e) => onChange({ ...settings, refreshIntervalSecs: Number(e.target.value) })}
            >
              {INTERVAL_OPTIONS.map((o) => (
                <option key={o.value} value={o.value}>{o.label}</option>
              ))}
            </select>
          </label>

          <div className="setting">
            <span className="setting-label">菜单栏展示</span>
            <div className="checkbox-row">
              {PLAN_OPTIONS.map((o) => (
                <label key={o.value} className="checkbox">
                  <input
                    type="checkbox"
                    checked={settings.trayPlans.includes(o.value)}
                    onChange={() => togglePlan(o.value)}
                  />
                  <span>{o.label} <em className="prefix">{o.prefix}</em></span>
                </label>
              ))}
            </div>
          </div>

          <label className="setting">
            <span className="setting-label">菜单栏周期</span>
            <select
              value={settings.trayPeriod}
              onChange={(e) => onChange({ ...settings, trayPeriod: e.target.value })}
            >
              {PERIOD_OPTIONS.map((o) => (
                <option key={o.value} value={o.value}>{o.label}</option>
              ))}
            </select>
          </label>

          <div className="setting">
            <span className="setting-label">
              告警阈值 · {settings.thresholdWarn}% / {settings.thresholdCritical}%
            </span>
            <div className="slider-row">
              <input
                type="range" min={0} max={100}
                value={settings.thresholdWarn}
                onChange={(e) => onChange({ ...settings, thresholdWarn: Number(e.target.value) })}
              />
              <input
                type="range" min={0} max={100}
                value={settings.thresholdCritical}
                onChange={(e) => onChange({ ...settings, thresholdCritical: Number(e.target.value) })}
              />
            </div>
            <div className="slider-legend">
              <span><i className="dot bar-ok" /> 正常</span>
              <span><i className="dot bar-warn" /> {settings.thresholdWarn}%+</span>
              <span><i className="dot bar-critical" /> {settings.thresholdCritical}%+</span>
            </div>
          </div>

          <label className="setting setting-inline">
            <span className="setting-label">开机自启</span>
            <input
              type="checkbox"
              checked={settings.autostart}
              onChange={(e) => onChange({ ...settings, autostart: e.target.checked })}
            />
          </label>
        </div>
      </div>
    </div>
  );
}
