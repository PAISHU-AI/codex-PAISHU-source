import { Ban, FolderOpen, Languages, Power, RefreshCw, Search, ShieldCheck, Trash2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import {
  archiveSkill,
  disableSkill,
  enableSkill,
  getSkillBoard,
  openSkillFolder,
  translateDescriptionToChinese,
} from "./api";
import type { SkillBoardData, SkillSummary } from "./types";
import "./SkillsBoard.css";

interface SkillsBoardProps {
  enabled: boolean;
}

const statusLabel: Record<SkillSummary["status"], string> = {
  enabled: "已启用",
  disabled: "已禁用",
  readOnly: "只读",
};

type SkillFilter = "all" | "enabled" | "disabled";

export function SkillsBoard({ enabled }: SkillsBoardProps) {
  const [board, setBoard] = useState<SkillBoardData | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<SkillFilter>("all");
  const [translationEnabled, setTranslationEnabled] = useState(false);
  const [translatedDescriptions, setTranslatedDescriptions] = useState<Record<string, string>>({});
  const [isTranslating, setIsTranslating] = useState(false);
  const [translationError, setTranslationError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isBusy, setIsBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadBoard = useCallback(async () => {
    if (!enabled) return;
    setIsLoading(true);
    setError(null);
    try {
      const next = await getSkillBoard();
      setBoard(next);
      setSelectedId((current) => current ?? next.skills[0]?.id ?? null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [enabled]);

  async function runAction(action: () => Promise<SkillBoardData | string>) {
    setIsBusy(true);
    setError(null);
    try {
      const result = await action();
      if (typeof result !== "string") {
        setBoard(result);
        setSelectedId((current) => {
          if (current && result.skills.some((skill) => skill.id === current)) {
            return current;
          }
          return result.skills[0]?.id ?? null;
        });
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsBusy(false);
    }
  }

  useEffect(() => {
    void loadBoard();
  }, [loadBoard]);

  const normalizedQuery = query.trim().toLowerCase();
  const skills = board?.skills ?? [];
  const statusFilteredSkills = skills.filter((skill) => {
    if (filter === "enabled") return skill.status !== "disabled";
    if (filter === "disabled") return skill.status === "disabled";
    return true;
  });
  const filteredSkills = normalizedQuery
    ? statusFilteredSkills.filter((skill) =>
        `${skill.name} ${skill.description} ${skill.sourceLabel}`
          .toLowerCase()
          .includes(normalizedQuery),
      )
    : statusFilteredSkills;
  const selectedSkill =
    filteredSkills.find((skill) => skill.id === selectedId) ?? filteredSkills[0] ?? null;
  const enabledCount = skills.filter((skill) => skill.status !== "disabled").length;
  const disabledCount = skills.filter((skill) => skill.status === "disabled").length;

  const translateSelectedDescription = useCallback(async () => {
    if (!selectedSkill || translatedDescriptions[selectedSkill.id]) {
      return;
    }
    setIsTranslating(true);
    setTranslationError(null);
    try {
      const translated = await translateDescriptionToChinese(selectedSkill.description);
      setTranslatedDescriptions((current) => ({ ...current, [selectedSkill.id]: translated }));
    } catch (err) {
      setTranslationError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsTranslating(false);
    }
  }, [selectedSkill, translatedDescriptions]);

  useEffect(() => {
    if (!translationEnabled) {
      setTranslationError(null);
      return;
    }
    void translateSelectedDescription();
  }, [translationEnabled, translateSelectedDescription]);

  const selectedDescription =
    selectedSkill && translationEnabled
      ? translatedDescriptions[selectedSkill.id] || selectedSkill.description
      : selectedSkill?.description;

  const isSelectedDescriptionTranslated =
    !!selectedSkill && translationEnabled && !!translatedDescriptions[selectedSkill.id];

  return (
    <section className="panel skills-board-panel">
      <div className="section-heading skills-board-heading">
        <div>
          <h2>Skills 技能看板</h2>
          <p>
            {enabled
              ? `${board?.totalCount ?? 0} 个技能 · ${board?.refreshedAt ?? "等待读取"}`
              : "已在设置中隐藏"}
          </p>
        </div>
        <div className="skills-board-heading-actions">
          <button
            className="quiet-button skills-board-translate"
            type="button"
            title={translationEnabled ? "取消翻译" : "使用 Google 翻译成中文"}
            aria-label={translationEnabled ? "取消翻译" : "翻译技能描述"}
            disabled={!enabled || !selectedSkill}
            onClick={() => setTranslationEnabled((current) => !current)}
          >
            <Languages size={15} />
            {translationEnabled ? "取消翻译" : "翻译"}
          </button>
          <button
            className="icon-button"
            type="button"
            title="刷新技能"
            aria-label="刷新技能"
            disabled={!enabled || isLoading || isBusy}
            onClick={() => void loadBoard()}
          >
            <RefreshCw size={15} className={isLoading ? "spin" : undefined} />
          </button>
        </div>
      </div>

      {!enabled ? (
        <div className="empty-state">Skills 看板已关闭。</div>
      ) : (
        <div className="skills-board-layout" aria-busy={isLoading || isBusy}>
          <div className="skills-board-sidebar">
            <label className="skills-board-search">
              <Search size={15} />
              <input
                value={query}
                placeholder="搜索技能"
                aria-label="搜索技能"
                onChange={(event) => setQuery(event.target.value)}
              />
            </label>

            <div className="skills-board-filters" aria-label="技能状态筛选">
              <FilterButton
                active={filter === "all"}
                label="全部"
                count={skills.length}
                onClick={() => setFilter("all")}
              />
              <FilterButton
                active={filter === "enabled"}
                label="已启用"
                count={enabledCount}
                onClick={() => setFilter("enabled")}
              />
              <FilterButton
                active={filter === "disabled"}
                label="已禁用"
                count={disabledCount}
                onClick={() => setFilter("disabled")}
              />
            </div>

            <div className="skills-board-list" role="listbox" aria-label="Skills 技能列表">
              {isLoading && !board ? (
                <div className="empty-state">正在读取技能...</div>
              ) : filteredSkills.length === 0 ? (
                <div className="empty-state">没有匹配的技能。</div>
              ) : (
                filteredSkills.map((skill) => (
                  <SkillListItem
                    key={skill.id}
                    skill={skill}
                    selected={skill.id === selectedSkill?.id}
                    disabled={isBusy}
                    onSelect={() => setSelectedId(skill.id)}
                    onOpenFolder={() => void runAction(() => openSkillFolder(skill.id))}
                    onDisable={() => {
                      void runAction(() => disableSkill(skill.id));
                    }}
                    onEnable={() => {
                      void runAction(async () => {
                        const result = await enableSkill(skill.id);
                        setFilter("enabled");
                        return result;
                      });
                    }}
                    onArchive={() => {
                      if (
                        !window.confirm(
                          `确认删除技能“${skill.name}”？该操作会移动到 skills-trash，不会永久删除。`,
                        )
                      ) {
                        return;
                      }
                      void runAction(() => archiveSkill(skill.id));
                    }}
                  />
                ))
              )}
            </div>
          </div>

          <SkillDetail
            skill={selectedSkill}
            description={selectedDescription ?? null}
            translated={isSelectedDescriptionTranslated}
            translating={isTranslating && !!selectedSkill}
            translationError={translationError}
          />
        </div>
      )}

      {(error || (board?.messages.length ?? 0) > 0) && (
        <div className="skills-board-message">
          {error && <strong>{error}</strong>}
          {board?.messages.map((message) => <span key={message}>{message}</span>)}
        </div>
      )}
    </section>
  );
}

function FilterButton({
  active,
  label,
  count,
  onClick,
}: {
  active: boolean;
  label: string;
  count: number;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={`skills-board-filter ${active ? "active" : ""}`}
      aria-pressed={active}
      onClick={onClick}
    >
      <span>{label}</span>
      <strong>{count}</strong>
    </button>
  );
}

function SkillListItem({
  skill,
  selected,
  disabled,
  onSelect,
  onOpenFolder,
  onDisable,
  onEnable,
  onArchive,
}: {
  skill: SkillSummary;
  selected: boolean;
  disabled: boolean;
  onSelect: () => void;
  onOpenFolder: () => void;
  onDisable: () => void;
  onEnable: () => void;
  onArchive: () => void;
}) {
  const canEnable = skill.status === "disabled";
  return (
    <article className={`skills-board-item ${selected ? "selected" : ""}`}>
      <button
        className="skills-board-item-main"
        type="button"
        role="option"
        aria-selected={selected}
        onClick={onSelect}
      >
        <span>{skill.name}</span>
        <small>{skill.sourceLabel}</small>
      </button>
      <div className="skills-board-actions">
        <button
          type="button"
          className="skills-board-action"
          title="打开文件夹"
          aria-label={`打开 ${skill.name} 文件夹`}
          disabled={disabled || !skill.canOpenFolder}
          onClick={onOpenFolder}
        >
          <FolderOpen size={14} />
        </button>
        {canEnable ? (
          <button
            type="button"
            className="skills-board-action state-disabled"
            title={skill.canEnable ? "启用" : "不可启用"}
            aria-label={`启用 ${skill.name}`}
            disabled={disabled || !skill.canEnable}
            onClick={onEnable}
          >
            <Power size={14} />
          </button>
        ) : (
          <button
            type="button"
            className="skills-board-action state-enabled"
            title={skill.canDisable ? "禁用" : "不可禁用"}
            aria-label={`禁用 ${skill.name}`}
            disabled={disabled || !skill.canDisable}
            onClick={onDisable}
          >
            <Ban size={14} />
          </button>
        )}
        <button
          type="button"
          className="skills-board-action danger"
          title={skill.canDelete ? "删除" : "不可删除"}
          aria-label={`删除 ${skill.name}`}
          disabled={disabled || !skill.canDelete}
          onClick={onArchive}
        >
          <Trash2 size={14} />
        </button>
      </div>
    </article>
  );
}

function SkillDetail({
  skill,
  description,
  translated,
  translating,
  translationError,
}: {
  skill: SkillSummary | null;
  description: string | null;
  translated: boolean;
  translating: boolean;
  translationError: string | null;
}) {
  if (!skill) {
    return (
      <div className="skills-board-detail empty">
        <ShieldCheck size={26} />
        <strong>暂无技能</strong>
        <span>没有读取到可显示的 Skills。</span>
      </div>
    );
  }

  return (
    <article className="skills-board-detail">
      <div className="skills-board-detail-header">
        <div>
          <span>
            {skill.sourceLabel}
            {translated ? " · Google 翻译" : ""}
          </span>
          <h3>{skill.name}</h3>
        </div>
        <strong>{statusLabel[skill.status]}</strong>
      </div>
      <p>{translating ? "正在使用 Google 翻译..." : description}</p>
      {translationError && <span className="skills-board-translation-error">{translationError}</span>}
      <dl>
        <div>
          <dt>技能目录</dt>
          <dd title={skill.folderPath}>{skill.folderPath}</dd>
        </div>
        <div>
          <dt>权限</dt>
          <dd>{skill.canDelete || skill.canDisable ? "用户技能，可管理" : "受保护，只读"}</dd>
        </div>
      </dl>
    </article>
  );
}
