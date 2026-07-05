import { useEffect, useRef, useState } from "react";
import type { PlanUsage, Settings } from "./lib/types";
import { DEFAULT_SETTINGS } from "./lib/settings";
import { invoke, isTauri, listen } from "./lib/runtime";
import { HeaderBar } from "./components/HeaderBar";
import { PlanCard } from "./components/PlanCard";
import { AuthBanner } from "./components/AuthBanner";
import { InstallBanner } from "./components/InstallBanner";
import { SettingsPanel } from "./components/SettingsPanel";

export default function App() {
  const [usage, setUsage] = useState<PlanUsage | null>(null);
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [refreshing, setRefreshing] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [logging, setLogging] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [now, setNow] = useState(() => Date.now());
  const contentRef = useRef<HTMLDivElement>(null);
  const refreshTick = useRef(0);

  // Initial load + event listeners.
  useEffect(() => {
    if (!isTauri()) return;
    const unlisteners: Array<() => void> = [];
    (async () => {
      try {
        const [u, s] = await Promise.all([
          invoke<PlanUsage>("get_usage"),
          invoke<Settings>("get_settings"),
        ]);
        setUsage(u);
        setSettings(s);
      } catch (e) {
        console.error("init failed", e);
      }
      unlisteners.push(
        await listen<PlanUsage>("usage-update", (p) => {
          setUsage(p);
          setRefreshing(false);
        }),
      );
      unlisteners.push(
        await listen<string>("tray-action", (a) => {
          if (a === "refresh") void doRefresh();
          else if (a === "open-settings") setSettingsOpen(true);
        }),
      );
    })();
    return () => unlisteners.forEach((fn) => fn());
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Live countdown tick (reset timers + next-refresh).
  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), 30000);
    return () => window.clearInterval(id);
  }, []);

  const doRefresh = async () => {
    if (!isTauri()) return;
    // Debounce: ignore if a refresh started within the last second.
    const t = Date.now();
    if (t - refreshTick.current < 1000) return;
    refreshTick.current = t;
    setRefreshing(true);
    try {
      await invoke<PlanUsage>("refresh_usage");
    } catch (e) {
      console.error(e);
      setRefreshing(false);
    }
  };

  const doLogin = async () => {
    if (!isTauri()) return;
    setLogging(true);
    try {
      await invoke("run_arkcli_login");
    } catch (e) {
      console.error(e);
    }
    setLogging(false);
  };

  const doInstall = async () => {
    if (!isTauri()) return;
    setInstalling(true);
    try {
      await invoke("run_arkcli_install");
    } catch (e) {
      console.error(e);
    }
    setInstalling(false);
  };

  // Keyboard shortcuts (popover must be focused).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (settingsOpen) {
          setSettingsOpen(false);
          e.preventDefault();
          return;
        }
        if (isTauri()) void invoke("hide_popover_cmd").catch(() => {});
        e.preventDefault();
        return;
      }
      if (!e.metaKey || e.ctrlKey || e.altKey || e.shiftKey) return;
      switch (e.key.toLowerCase()) {
        case "r":
          void doRefresh();
          e.preventDefault();
          break;
        case ",":
          setSettingsOpen(true);
          e.preventDefault();
          break;
        case "w":
          if (isTauri()) void invoke("hide_popover_cmd").catch(() => {});
          e.preventDefault();
          break;
        case "q":
          if (isTauri()) void invoke("quit_app").catch(() => {});
          e.preventDefault();
          break;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [settingsOpen]);

  // Fit the native window to content height.
  useEffect(() => {
    if (!isTauri() || !contentRef.current) return;
    const el = contentRef.current;
    let raf = 0;
    const push = () => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(async () => {
        const h = el.getBoundingClientRect().height;
        try {
          await invoke("set_popover_height", { height: Math.ceil(h + 2) });
        } catch {
          /* window may be hidden */
        }
      });
    };
    push();
    const ro = new ResizeObserver(push);
    ro.observe(el);
    let unlisten = () => {};
    listen("popover-shown", () => push())
      .then((fn) => (unlisten = fn))
      .catch(() => {});
    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      unlisten();
    };
  }, [usage, settingsOpen]);

  const saveSettings = (s: Settings) => {
    setSettings(s);
    if (isTauri()) void invoke("set_settings", { settings: s }).catch(() => {});
  };

  const notInstalled = usage?.notInstalled ?? false;
  const authExpired = !notInstalled && (usage?.authExpired ?? false);
  const hasError = !notInstalled && !!usage?.error && !authExpired;

  const fetchedMs = usage?.fetchedAt ? new Date(usage.fetchedAt).getTime() : 0;
  const remainMin = Math.ceil(
    Math.max(0, fetchedMs + settings.refreshIntervalSecs * 1000 - now) / 60000,
  );

  return (
    <div className="page">
      <div className="page-content" ref={contentRef}>
        {notInstalled && (
          <InstallBanner onInstall={doInstall} installing={installing} />
        )}
        {authExpired && (
          <AuthBanner message={usage?.error} onLogin={doLogin} logging={logging} />
        )}
        <HeaderBar
          viewer={usage?.viewer ?? null}
          fetchedAt={usage?.fetchedAt ?? ""}
          refreshing={refreshing}
          onRefresh={doRefresh}
          onOpenSettings={() => setSettingsOpen(true)}
        />
        {hasError && <div className="error-banner">⚠ {usage?.error}</div>}
        {!usage && !hasError && <div className="loading">加载中…</div>}
        {usage && usage.plans.length > 0 && (
          <div className="cards">
            {usage.plans.map((p) => (
              <PlanCard
                key={p.product}
                plan={p}
                warn={settings.thresholdWarn}
                critical={settings.thresholdCritical}
                now={now}
              />
            ))}
          </div>
        )}
        {usage && usage.plans.length === 0 && !notInstalled && !authExpired && !hasError && (
          <div className="empty">没有订阅的套餐</div>
        )}
        <div className="footer">
          <span>{usage ? `${remainMin} 分钟后刷新` : "—"}</span>
          <span className="footer-hint">Ctrl+⌘+V 唤出</span>
        </div>
      </div>
      <SettingsPanel
        open={settingsOpen}
        settings={settings}
        onChange={saveSettings}
        onClose={() => setSettingsOpen(false)}
      />
    </div>
  );
}
