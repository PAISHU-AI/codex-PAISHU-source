import { useEffect, useState } from "react";
import { RotateCcw, Save, Trash2, X } from "lucide-react";
import {
  createCodexConfigBackup,
  deleteCodexConfigBackup,
  getDetectionPaths,
  listCodexConfigBackups,
  restoreCodexConfigBackup,
} from "../lib/api";
import type {
  ApiSpeedMode,
  AppSettings,
  CodexAccessMode,
  CodexConfigBackup,
  DetectionPaths,
  LanguageMode,
  ReasoningEffort,
  ThemeMode,
} from "../types/usage";

interface SettingsDrawerProps {
  settings: AppSettings;
  onClose: () => void;
  onSave: (settings: AppSettings) => Promise<void> | void;
}

export function SettingsDrawer({ settings, onClose, onSave }: SettingsDrawerProps) {
  const [draft, setDraft] = useState(settings);
  const [paths, setPaths] = useState<DetectionPaths | null>(null);
  const [backups, setBackups] = useState<CodexConfigBackup[]>([]);
  const [selectedBackupId, setSelectedBackupId] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [isBackupBusy, setIsBackupBusy] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [backupStatus, setBackupStatus] = useState<string | null>(null);
  const selectedBackup = backups.find((backup) => backup.id === selectedBackupId) ?? null;

  useEffect(() => {
    void getDetectionPaths().then(setPaths);
    void refreshBackups();
  }, []);

  async function refreshBackups() {
    try {
      const next = await listCodexConfigBackups();
      setBackups(next);
      setSelectedBackupId((current) =>
        current && next.some((backup) => backup.id === current) ? current : next[0]?.id || "",
      );
    } catch (err) {
      setBackupStatus(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleSave() {
    setIsSaving(true);
    setSaveError(null);
    try {
      await onSave(draft);
      onClose();
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : String(err));
      setIsSaving(false);
    }
  }

  async function handleCreateBackup() {
    setIsBackupBusy(true);
    setSaveError(null);
    setBackupStatus(null);
    try {
      const next = await createCodexConfigBackup();
      setBackups(next);
      setSelectedBackupId(next[0]?.id || "");
      setBackupStatus("已保存当前 Codex 配置备份");
    } catch (err) {
      setBackupStatus(err instanceof Error ? err.message : String(err));
    } finally {
      setIsBackupBusy(false);
    }
  }

  async function handleRestoreBackup() {
    if (!selectedBackupId) return;
    setIsBackupBusy(true);
    setSaveError(null);
    setBackupStatus(null);
    try {
      const next = await restoreCodexConfigBackup(selectedBackupId);
      setBackups(next);
      setBackupStatus("已恢复所选 Codex 配置备份，建议重启 Codex");
    } catch (err) {
      setBackupStatus(err instanceof Error ? err.message : String(err));
    } finally {
      setIsBackupBusy(false);
    }
  }

  async function handleDeleteBackup() {
    if (!selectedBackupId || selectedBackup?.isDefault) return;
    setIsBackupBusy(true);
    setSaveError(null);
    setBackupStatus(null);
    try {
      const next = await deleteCodexConfigBackup(selectedBackupId);
      setBackups(next);
      setSelectedBackupId(next[0]?.id || "");
      setBackupStatus("已删除所选配置备份");
    } catch (err) {
      setBackupStatus(err instanceof Error ? err.message : String(err));
    } finally {
      setIsBackupBusy(false);
    }
  }

  return (
    <aside className="settings-drawer" aria-label="设置">
      <div className="drawer-header">
        <div>
          <h2>设置</h2>
        </div>
        <button className="icon-button" onClick={onClose} aria-label="关闭设置">
          <X size={16} />
        </button>
      </div>

      <section className="settings-section" aria-label="接入方式">
        <h3>接入方式</h3>
        <div className="config-backup-panel">
          <label>
            配置备份
            <select
              value={selectedBackupId}
              onChange={(event) => setSelectedBackupId(event.target.value)}
              disabled={isBackupBusy || backups.length === 0}
            >
              {backups.length === 0 ? (
                <option value="">暂无备份</option>
              ) : (
                backups.map((backup) => (
                  <option key={backup.id} value={backup.id}>
                    {backup.isDefault ? "默认配置 - " : ""}
                    {backup.label}
                    {backup.createdAt ? ` · ${formatBackupTime(backup.createdAt)}` : ""}
                  </option>
                ))
              )}
            </select>
          </label>
          <div className="config-backup-actions">
            <button
              className="quiet-button"
              type="button"
              disabled={isBackupBusy}
              onClick={() => void handleCreateBackup()}
            >
              <Save size={14} />
              保存备份
            </button>
            <button
              className="quiet-button"
              type="button"
              disabled={isBackupBusy || !selectedBackupId}
              onClick={() => void handleRestoreBackup()}
            >
              <RotateCcw size={14} />
              恢复备份
            </button>
            <button
              className="quiet-button danger-button"
              type="button"
              disabled={isBackupBusy || !selectedBackupId || !!selectedBackup?.isDefault}
              onClick={() => void handleDeleteBackup()}
            >
              <Trash2 size={14} />
              删除备份
            </button>
          </div>
          <p className="settings-hint">
            首次启动会自动保存默认配置；恢复前会额外保存当前配置，避免误恢复后无法回退。
          </p>
          {backupStatus && <p className="settings-backup-status">{backupStatus}</p>}
        </div>
        <label>
          当前模式
          <select
            value={draft.accessMode}
            onChange={(event) =>
              setDraft(applyAccessMode(draft, event.target.value as CodexAccessMode))
            }
          >
            <option value="official">官方原生</option>
            <option value="relay">API 中转</option>
          </select>
        </label>
        <p className="settings-hint">{accessModeHint(draft.accessMode)}</p>

        {draft.accessMode === "relay" && (
          <>
            <label>
              API 地址
              <input
                value={draft.apiEndpoint ?? ""}
                placeholder="https://api.example.com"
                onBlur={(event) =>
                  setDraft({ ...draft, apiEndpoint: normalizeApiEndpoint(event.target.value) })
                }
                onChange={(event) =>
                  setDraft({ ...draft, apiEndpoint: event.target.value || null })
                }
              />
            </label>

            <label>
              API Key
              <input
                type="password"
                autoComplete="off"
                value={draft.apiKey ?? ""}
                placeholder="首次配置必填；留空会保留已有 Key"
                onChange={(event) => setDraft({ ...draft, apiKey: event.target.value || null })}
              />
            </label>

            <label>
              模型名字
              <input
                value={draft.apiModel}
                placeholder="gpt-5"
                onChange={(event) => setDraft({ ...draft, apiModel: event.target.value })}
              />
            </label>

            <div className="settings-inline-grid">
              <label>
                推理强度
                <select
                  value={draft.reasoningEffort}
                  onChange={(event) =>
                    setDraft({ ...draft, reasoningEffort: event.target.value as ReasoningEffort })
                  }
                >
                  <option value="minimal">极低</option>
                  <option value="low">低</option>
                  <option value="medium">中</option>
                  <option value="high">高</option>
                  <option value="extreme">超高</option>
                </select>
              </label>

              <label>
                速度策略
                <select
                  value={draft.speedMode}
                  onChange={(event) =>
                    setDraft({ ...draft, speedMode: event.target.value as ApiSpeedMode })
                  }
                >
                  <option value="stable">稳定</option>
                  <option value="balanced">均衡</option>
                  <option value="fast">快速</option>
                </select>
              </label>
            </div>
            <p className="settings-hint">
              保存后会同步 API 地址、API Key、模型、推理强度和速度策略到 Codex 配置；快速模式使用
              service_tier=priority，稳定/均衡不强制服务层。切换认证方式后建议重启 Codex。
            </p>
          </>
        )}
      </section>

      <section className="settings-section" aria-label="会员计划">
        <h3>会员计划</h3>
        <label>
          会员计划起算日
          <input
            type="date"
            value={draft.membershipStartedOn ?? ""}
            onChange={(event) =>
              setDraft({ ...draft, membershipStartedOn: event.target.value || null })
            }
          />
        </label>
        <p className="settings-hint">
          填写开通会员的日期即可，统计会自动按当前会员周期起点累计。例如 4/10 开通，7/3 时按 6/10
          至今日累计。
        </p>
      </section>

      <section className="settings-section" aria-label="界面偏好">
        <h3>界面偏好</h3>
        <label>
          语言
          <select
            value={draft.language}
            onChange={(event) =>
              setDraft({ ...draft, language: event.target.value as LanguageMode })
            }
          >
            <option value="auto">跟随系统</option>
            <option value="zh">中文</option>
            <option value="en">英文</option>
          </select>
        </label>

        <label>
          主题
          <select
            value={draft.theme}
            onChange={(event) => setDraft({ ...draft, theme: event.target.value as ThemeMode })}
          >
            <option value="system">跟随系统</option>
            <option value="light">浅色</option>
            <option value="dark">深色</option>
          </select>
        </label>
      </section>

      <label>
        刷新间隔（秒）
        <input
          type="number"
          min={30}
          max={3600}
          value={draft.refreshIntervalSecs}
          onChange={(event) =>
            setDraft({ ...draft, refreshIntervalSecs: Number(event.target.value) || 300 })
          }
        />
      </label>

      <label>
        Codex 可执行文件路径
        <input
          value={draft.codexBinaryPath ?? ""}
          placeholder={paths?.codexBinaryPath ?? "自动从 PATH 探测"}
          onChange={(event) => setDraft({ ...draft, codexBinaryPath: event.target.value || null })}
        />
      </label>

      <label>
        Codex 数据目录
        <input
          value={draft.codexDataDir ?? ""}
          placeholder={paths?.codexDataDir ?? "自动探测 ~/.codex"}
          onChange={(event) => setDraft({ ...draft, codexDataDir: event.target.value || null })}
        />
      </label>

      <div className="toggle-row">
        <label>
          <input
            type="checkbox"
            checked={draft.alwaysOnTop}
            onChange={(event) => setDraft({ ...draft, alwaysOnTop: event.target.checked })}
          />
          窗口置顶
        </label>
        <label>
          <input
            type="checkbox"
            checked={draft.showTaskBoard}
            onChange={(event) => setDraft({ ...draft, showTaskBoard: event.target.checked })}
          />
          显示 Skills 看板
        </label>
        <label>
          <input
            type="checkbox"
            checked={draft.showOnStart}
            onChange={(event) => setDraft({ ...draft, showOnStart: event.target.checked })}
          />
          启动时显示
        </label>
      </div>

      <div className="path-summary">
        <strong>已探测状态数据库</strong>
        <span>{paths?.stateDbPath ?? "未探测到"}</span>
      </div>

      <div className="drawer-actions">
        {saveError && (
          <p className="settings-error" role="alert">
            {saveError}
          </p>
        )}
        <button className="primary-button" disabled={isSaving} onClick={() => void handleSave()}>
          {isSaving ? "保存中" : "保存设置"}
        </button>
      </div>
    </aside>
  );
}

function accessModeHint(mode: CodexAccessMode): string {
  if (mode === "relay") {
    return "填写基础 API 地址和 API Key；保存时会自动补全为单个 /v1，并同步 Codex 配置和认证文件。";
  }
  return "官方原生模式不需要 API 地址；保存设置会恢复 Codex 官方登录配置。切换后建议重启 Codex。";
}

function applyAccessMode(settings: AppSettings, accessMode: CodexAccessMode): AppSettings {
  if (accessMode === "official") {
    return {
      ...settings,
      accessMode,
      apiEndpoint: null,
      apiKey: null,
      apiModel: "gpt-5",
      reasoningEffort: "medium",
      speedMode: "balanced",
    };
  }
  return { ...settings, accessMode };
}

function normalizeApiEndpoint(value: string): string | null {
  const trimmed = value.trim().replace(/\/+$/, "");
  if (!trimmed) return null;
  const withScheme = /^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
  const canonical = withScheme.replace(/(?:\/v1)+$/i, "/v1");
  return /\/v1$/i.test(canonical) ? canonical : `${canonical}/v1`;
}

function formatBackupTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}
