import { useEffect, useState } from "react";
import { getAppSettings, getUsageSnapshot, saveAppSettings, setAlwaysOnTop } from "./lib/api";
import { selectDashboardData } from "./lib/dashboardData";
import { defaultSettings } from "./lib/mock";
import type { AppSettings, UsageSnapshot } from "./types/usage";
import { EnvironmentPanel } from "./components/EnvironmentPanel";
import { ErrorDisclosure } from "./components/ErrorDisclosure";
import { HeaderBar } from "./components/HeaderBar";
import { LoginStatusCard } from "./components/LoginStatusCard";
import { QuotaPanel } from "./components/QuotaPanel";
import { SettingsDrawer } from "./components/SettingsDrawer";
import { TokenValuePanel } from "./components/TokenValuePanel";
import { TrendDetailDialog } from "./components/TrendDetailDialog";
import { TrendPanel } from "./components/TrendPanel";
import { SkillsBoard } from "./features/skills-board/SkillsBoard";
import { KnowledgeBoard } from "./features/knowledge-board/KnowledgeBoard";

export function App() {
  const [snapshot, setSnapshot] = useState<UsageSnapshot | null>(null);
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [trendDetailOpen, setTrendDetailOpen] = useState(false);

  async function refresh() {
    setIsRefreshing(true);
    setError(null);
    try {
      setSnapshot(await getUsageSnapshot());
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsRefreshing(false);
    }
  }

  async function updateSettings(next: AppSettings) {
    const saved = await saveAppSettings(next);
    setSettings(saved);
    document.documentElement.dataset.theme = saved.theme;
    try {
      await setAlwaysOnTop(saved.alwaysOnTop);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
    void refresh();
  }

  useEffect(() => {
    getAppSettings()
      .then((loaded) => {
        setSettings(loaded);
        document.documentElement.dataset.theme = loaded.theme;
      })
      .catch(() => undefined);
    void refresh();
  }, []);

  useEffect(() => {
    if (settings.refreshIntervalSecs <= 0) return;
    const id = window.setInterval(() => void refresh(), settings.refreshIntervalSecs * 1000);
    return () => window.clearInterval(id);
  }, [settings.refreshIntervalSecs]);

  const hasPartialData = !!snapshot && snapshot.diagnostics.some((item) => item.status !== "ok");
  const dashboardData = selectDashboardData(snapshot, settings.accessMode);

  return (
    <main className="app-shell">
      <HeaderBar
        snapshot={snapshot}
        isRefreshing={isRefreshing}
        onRefresh={() => void refresh()}
        onOpenSettings={() => setSettingsOpen(true)}
      />
      <section className="dashboard-grid" aria-busy={isRefreshing}>
        <QuotaPanel
          snapshot={snapshot}
          isLoading={!snapshot && isRefreshing}
          accessMode={settings.accessMode}
        />
        <TokenValuePanel
          usage={dashboardData.tokenUsage}
          valuePeriodUsage={dashboardData.valuePeriodUsage}
          isLoading={!snapshot}
          sourceLabel={dashboardData.tokenSourceLabel}
        />
        <TrendPanel
          buckets={dashboardData.trendBuckets}
          isLoading={!snapshot}
          sourceLabel={dashboardData.trendSourceLabel}
          onOpenDetail={() => setTrendDetailOpen(true)}
        />
        <LoginStatusCard snapshot={snapshot} settings={settings} />
        <KnowledgeBoard />
        <SkillsBoard enabled={settings.showTaskBoard} />
        <EnvironmentPanel diagnostics={snapshot?.diagnostics ?? []} isPartial={hasPartialData} />
      </section>
      <ErrorDisclosure
        title="运行消息"
        messages={[...(snapshot?.messages ?? []), error].filter(Boolean) as string[]}
      />
      {settingsOpen && (
        <SettingsDrawer
          settings={settings}
          onClose={() => setSettingsOpen(false)}
          onSave={updateSettings}
        />
      )}
      {trendDetailOpen && (
        <TrendDetailDialog
          buckets={dashboardData.trendDetailBuckets}
          sourceLabel={dashboardData.trendSourceLabel}
          onClose={() => setTrendDetailOpen(false)}
        />
      )}
    </main>
  );
}
