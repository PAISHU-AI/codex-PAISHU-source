import {
  BookOpenText,
  CheckCircle2,
  CircleSlash2,
  Clock3,
  Database,
  FileText,
  FolderOpen,
  Languages,
  Layers3,
  LoaderCircle,
  Power,
  RefreshCw,
  Search,
  Trash2,
  XCircle,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  deleteKnowledge,
  getKnowledgeBoard,
  getKnowledgeOverview,
  openKnowledgeSource,
  setKnowledgeEnabled,
  syncKnowledgeSources,
  translateKnowledgeOverviewToChinese,
} from "./api";
import type { KnowledgeBoardData, KnowledgeDocumentSummary, KnowledgeOverview } from "./types";
import "./KnowledgeBoard.css";

type KnowledgeFilter = "all" | "enabled" | "disabled";

export const KNOWLEDGE_STATUS_REFRESH_INTERVAL_MS = 200_000;

export function KnowledgeBoard() {
  const [board, setBoard] = useState<KnowledgeBoardData | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<KnowledgeFilter>("enabled");
  const [isLoading, setIsLoading] = useState(false);
  const [isBusy, setIsBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [overview, setOverview] = useState<KnowledgeOverview | null>(null);
  const [isOverviewLoading, setIsOverviewLoading] = useState(false);
  const [overviewError, setOverviewError] = useState<string | null>(null);
  const [translationEnabled, setTranslationEnabled] = useState(false);
  const [translatedOverview, setTranslatedOverview] = useState<string | null>(null);
  const [isTranslating, setIsTranslating] = useState(false);
  const [translationError, setTranslationError] = useState<string | null>(null);
  const refreshInFlight = useRef(false);

  const loadBoard = useCallback(async (mode: "sync" | "status" = "sync") => {
    if (refreshInFlight.current) return;
    refreshInFlight.current = true;
    setIsLoading(true);
    setError(null);
    try {
      const next = mode === "sync" ? await syncKnowledgeSources() : await getKnowledgeBoard();
      setBoard(next);
      setSelectedId((current) => current ?? next.documents[0]?.id ?? null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      refreshInFlight.current = false;
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadBoard();
  }, [loadBoard]);

  useEffect(() => {
    const id = window.setInterval(
      () => void loadBoard("status"),
      KNOWLEDGE_STATUS_REFRESH_INTERVAL_MS,
    );
    return () => window.clearInterval(id);
  }, [loadBoard]);

  const documents = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return (board?.documents ?? [])
      .filter((document) => {
        if (filter === "enabled" && !document.enabled) return false;
        if (filter === "disabled" && document.enabled) return false;
        if (!normalizedQuery) return true;
        return `${document.title} ${document.packageName} ${document.owner} ${document.sourceUri}`
          .toLowerCase()
          .includes(normalizedQuery);
      })
      .sort(
        (left, right) =>
          left.packageName.localeCompare(right.packageName, "zh-CN") ||
          left.title.localeCompare(right.title, "zh-CN"),
      );
  }, [board, filter, query]);

  const selectedDocument =
    documents.find((document) => document.id === selectedId) ?? documents[0] ?? null;

  useEffect(() => {
    const documentId = selectedDocument?.id;
    setOverview(null);
    setOverviewError(null);
    setTranslationEnabled(false);
    setTranslatedOverview(null);
    setTranslationError(null);
    if (!documentId) return;

    let cancelled = false;
    setIsOverviewLoading(true);
    void getKnowledgeOverview(documentId)
      .then((next) => {
        if (!cancelled) setOverview(next);
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setOverviewError(err instanceof Error ? err.message : String(err));
        }
      })
      .finally(() => {
        if (!cancelled) setIsOverviewLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [selectedDocument?.id]);

  async function toggleOverviewTranslation() {
    if (translationEnabled) {
      setTranslationEnabled(false);
      setTranslationError(null);
      return;
    }
    if (!overview) return;
    if (translatedOverview) {
      setTranslationEnabled(true);
      return;
    }
    setIsTranslating(true);
    setTranslationError(null);
    try {
      const translated = await translateKnowledgeOverviewToChinese(overview.overview);
      setTranslatedOverview(translated);
      setTranslationEnabled(true);
    } catch (err) {
      setTranslationError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsTranslating(false);
    }
  }

  async function toggleKnowledge(document: KnowledgeDocumentSummary) {
    setIsBusy(true);
    setError(null);
    try {
      const next = await setKnowledgeEnabled(document.id, !document.enabled);
      setBoard(next);
      setSelectedId(document.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsBusy(false);
    }
  }

  async function revealKnowledgeSource(document: KnowledgeDocumentSummary) {
    setIsBusy(true);
    setError(null);
    try {
      await openKnowledgeSource(document.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsBusy(false);
    }
  }

  async function removeKnowledge(document: KnowledgeDocumentSummary) {
    if (
      !window.confirm(
        `确认删除知识“${document.title}”？源文件将移至知识回收站，向量检索会立即停用。`,
      )
    ) {
      return;
    }
    setIsBusy(true);
    setError(null);
    try {
      const next = await deleteKnowledge(document.id);
      setBoard(next);
      setSelectedId(next.documents[0]?.id ?? null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsBusy(false);
    }
  }

  return (
    <section className="panel knowledge-board-panel">
      <div className="section-heading knowledge-board-heading">
        <div>
          <h2>知识库可视化</h2>
          <p>
            {board
              ? `${board.collectionCount} 个集合 · ${board.totalDocuments} 份知识 · ${formatRefreshTime(board.refreshedAt)}`
              : "连接 PAISHU 向量知识服务"}
          </p>
        </div>
        <div className="knowledge-heading-actions">
          <span
            className={`knowledge-service-state ${board?.serviceStatus === "ok" ? "online" : "offline"}`}
          >
            <span />
            {board?.serviceStatus === "ok" ? "服务正常" : "等待连接"}
          </span>
          <span
            className={`knowledge-auto-monitor ${isLoading ? "refreshing" : ""}`}
            aria-label="知识库自动监测，每 200 秒刷新一次状态"
            title="每 200 秒自动读取知识库状态，不会重复写入知识库"
          >
            <RefreshCw size={12} className={isLoading ? "spin" : undefined} />
            {isLoading ? "监测中 · 200 秒" : "自动监测 · 200 秒"}
          </span>
          <button
            className="icon-button"
            type="button"
            aria-label="刷新知识库"
            title="刷新知识库"
            disabled={isLoading || isBusy}
            onClick={() => void loadBoard()}
          >
            <RefreshCw size={15} className={isLoading ? "spin" : undefined} />
          </button>
        </div>
      </div>

      <div className="knowledge-metrics">
        <Metric
          icon={<FileText size={16} />}
          label="知识清单"
          value={`${board?.totalDocuments ?? 0} 份知识`}
          detail={`${board?.enabledDocuments ?? 0} 启用 · ${board?.disabledDocuments ?? 0} 禁用`}
        />
        <Metric
          icon={<Layers3 size={16} />}
          label="向量分块"
          value={`${board?.chunkCount ?? 0} 个分块`}
          detail="pgvector HNSW 索引"
        />
        <Metric
          icon={<Database size={16} />}
          label="知识库容量"
          value={formatBytes(board?.databaseBytes ?? 0)}
          detail="PostgreSQL 实际占用"
        />
        <Metric
          icon={<Clock3 size={16} />}
          label="平均读取时长"
          value={`${board?.averageReadMs ?? 0} ms`}
          detail="向量＋关键词混合检索"
        />
        <Metric
          icon={<CheckCircle2 size={16} />}
          label="读取成功"
          value={formatInteger(board?.readSuccessCount ?? 0)}
          detail="累计成功请求"
          tone="success"
        />
        <Metric
          icon={<XCircle size={16} />}
          label="读取失败"
          value={formatInteger(board?.readFailureCount ?? 0)}
          detail="累计失败请求"
          tone="danger"
        />
      </div>

      <div className="knowledge-board-layout" aria-busy={isLoading || isBusy}>
        <div className="knowledge-sidebar">
          <label className="knowledge-search">
            <Search size={15} />
            <input
              aria-label="搜索知识"
              placeholder="搜索知识标题、来源或负责人"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
          </label>
          <div className="knowledge-filters" aria-label="知识状态筛选">
            <FilterButton
              label="全部"
              count={board?.totalDocuments ?? 0}
              active={filter === "all"}
              onClick={() => setFilter("all")}
            />
            <FilterButton
              label="已启用"
              count={board?.enabledDocuments ?? 0}
              active={filter === "enabled"}
              onClick={() => setFilter("enabled")}
            />
            <FilterButton
              label="已禁用"
              count={board?.disabledDocuments ?? 0}
              active={filter === "disabled"}
              onClick={() => setFilter("disabled")}
            />
          </div>
          <div className="knowledge-list" role="listbox" aria-label="知识库清单">
            {isLoading && !board ? (
              <div className="empty-state">正在读取向量知识库...</div>
            ) : documents.length === 0 ? (
              <div className="empty-state">没有匹配的知识。</div>
            ) : (
              documents.map((document) => (
                <KnowledgeListItem
                  key={document.id}
                  document={document}
                  selected={document.id === selectedDocument?.id}
                  disabled={isBusy}
                  onSelect={() => setSelectedId(document.id)}
                  onOpen={() => void revealKnowledgeSource(document)}
                  onToggle={() => void toggleKnowledge(document)}
                  onDelete={() => void removeKnowledge(document)}
                />
              ))
            )}
          </div>
        </div>
        <KnowledgeDetail
          document={selectedDocument}
          overview={overview}
          overviewText={
            translationEnabled && translatedOverview
              ? translatedOverview
              : (overview?.overview ?? null)
          }
          overviewLoading={isOverviewLoading}
          overviewError={overviewError}
          translated={translationEnabled && !!translatedOverview}
          translating={isTranslating}
          translationError={translationError}
          onToggleTranslation={() => void toggleOverviewTranslation()}
        />
      </div>

      {(error || (board?.messages.length ?? 0) > 0) && (
        <div className="knowledge-message">
          {error && <strong>{error}</strong>}
          {board?.messages.map((message) => (
            <span key={message}>{message}</span>
          ))}
        </div>
      )}
    </section>
  );
}

function Metric({
  icon,
  label,
  value,
  detail,
  tone = "default",
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  detail: string;
  tone?: "default" | "success" | "danger";
}) {
  return (
    <article className={`knowledge-metric ${tone}`}>
      <span className="knowledge-metric-icon">{icon}</span>
      <div>
        <span>{label}</span>
        <strong>{value}</strong>
        <small>{detail}</small>
      </div>
    </article>
  );
}

function FilterButton({
  label,
  count,
  active,
  onClick,
}: {
  label: string;
  count: number;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      className={`knowledge-filter ${active ? "active" : ""}`}
      type="button"
      aria-pressed={active}
      onClick={onClick}
    >
      <span>{label}</span>
      <strong>{count}</strong>
    </button>
  );
}

function KnowledgeListItem({
  document,
  selected,
  disabled,
  onSelect,
  onOpen,
  onToggle,
  onDelete,
}: {
  document: KnowledgeDocumentSummary;
  selected: boolean;
  disabled: boolean;
  onSelect: () => void;
  onOpen: () => void;
  onToggle: () => void;
  onDelete: () => void;
}) {
  return (
    <div
      className={`knowledge-item ${selected ? "selected" : ""} ${document.enabled ? "enabled" : "disabled"}`}
      role="option"
      aria-selected={selected}
    >
      <button className="knowledge-item-main" type="button" onClick={onSelect}>
        <span>{document.title}</span>
        <small title={document.packageName}>
          {document.packageName || "未分类知识"} · {document.chunkCount} 分块 ·{" "}
          {formatInteger(document.approximateTokens)} tokens
        </small>
      </button>
      <div className="knowledge-item-actions">
        <button
          className="knowledge-item-action"
          type="button"
          disabled={disabled}
          aria-label={`定位 ${document.title}`}
          title="在文件管理器中定位源文件"
          onClick={onOpen}
        >
          <FolderOpen size={14} />
        </button>
        <button
          className={`knowledge-item-action ${document.enabled ? "state-enabled" : "state-disabled"}`}
          type="button"
          disabled={disabled}
          aria-label={`${document.enabled ? "禁用" : "启用"} ${document.title}`}
          title={document.enabled ? "禁用知识" : "启用知识"}
          onClick={onToggle}
        >
          {document.enabled ? <Power size={14} /> : <CircleSlash2 size={14} />}
        </button>
        <button
          className="knowledge-item-action danger"
          type="button"
          disabled={disabled}
          aria-label={`删除 ${document.title}`}
          title="删除知识"
          onClick={onDelete}
        >
          <Trash2 size={14} />
        </button>
      </div>
    </div>
  );
}

function KnowledgeDetail({
  document,
  overview,
  overviewText,
  overviewLoading,
  overviewError,
  translated,
  translating,
  translationError,
  onToggleTranslation,
}: {
  document: KnowledgeDocumentSummary | null;
  overview: KnowledgeOverview | null;
  overviewText: string | null;
  overviewLoading: boolean;
  overviewError: string | null;
  translated: boolean;
  translating: boolean;
  translationError: string | null;
  onToggleTranslation: () => void;
}) {
  if (!document)
    return (
      <div className="knowledge-detail empty">
        <Database size={24} />
        <strong>选择一份知识</strong>
        <span>查看向量分块、权限、来源和容量信息。</span>
      </div>
    );
  return (
    <div className="knowledge-detail">
      <div className="knowledge-detail-header">
        <div>
          <span>知识详情</span>
          <h3>{document.title}</h3>
        </div>
        <strong className={document.enabled ? "enabled" : "disabled"}>
          {document.enabled ? "已启用" : "已禁用"}
        </strong>
      </div>
      <div className="knowledge-detail-grid">
        <Detail label="向量分块" value={`${document.chunkCount} 个`} />
        <Detail label="估算 Tokens" value={formatInteger(document.approximateTokens)} />
        <Detail label="访问级别" value={accessTierLabel(document.accessTier)} />
        <Detail label="知识状态" value={document.status} />
      </div>
      <section className="knowledge-overview" aria-busy={overviewLoading || translating}>
        <div className="knowledge-overview-header">
          <div>
            <BookOpenText size={15} />
            <span>知识概述</span>
            {overview && <small>{translated ? "简体中文" : "原文 · " + overview.language}</small>}
          </div>
          <button
            className="quiet-button knowledge-overview-translate"
            type="button"
            aria-label={translated ? "取消知识概述翻译" : "翻译知识概述"}
            title={translated ? "恢复原文" : "使用 Google 翻译成简体中文"}
            disabled={!overview || overviewLoading || translating}
            onClick={onToggleTranslation}
          >
            {translating ? <LoaderCircle size={14} className="spin" /> : <Languages size={14} />}
            {translated ? "取消翻译" : "翻译"}
          </button>
        </div>
        {overviewLoading ? (
          <p className="knowledge-overview-placeholder">正在读取当前知识正文...</p>
        ) : overviewError ? (
          <p className="knowledge-overview-error">{overviewError}</p>
        ) : (
          <p>{overviewText || "当前知识没有可显示的概述。"}</p>
        )}
        {translationError && <span className="knowledge-overview-error">{translationError}</span>}
      </section>
      <dl>
        <div>
          <dt>来源</dt>
          <dd title={document.sourceUri}>{document.sourceUri}</dd>
        </div>
        <div>
          <dt>负责人</dt>
          <dd>{document.owner}</dd>
        </div>
        <div>
          <dt>最后更新</dt>
          <dd>{formatRefreshTime(document.updatedAt)}</dd>
        </div>
      </dl>
    </div>
  );
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return `${(bytes / 1024 ** index).toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
}
function formatInteger(value: number): string {
  return new Intl.NumberFormat("zh-CN").format(value);
}
function formatRefreshTime(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN", { hour12: false });
}
function accessTierLabel(value: string): string {
  return value === "confidential" ? "机密" : value === "public" ? "公开" : "内部";
}
