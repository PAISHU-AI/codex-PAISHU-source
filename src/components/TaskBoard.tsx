import { CheckCircle2, Circle, Clock3, Radio } from "lucide-react";
import type { ReactNode } from "react";
import type { TaskBoard as TaskBoardType, TaskColumn, TaskColumnKind } from "../types/usage";

interface TaskBoardProps {
  board: TaskBoardType | null;
  enabled: boolean;
}

const icons: Record<TaskColumnKind, ReactNode> = {
  active: <Radio size={15} />,
  pending: <Circle size={15} />,
  scheduled: <Clock3 size={15} />,
  done: <CheckCircle2 size={15} />,
};

export function TaskBoard({ board, enabled }: TaskBoardProps) {
  return (
    <section className="panel task-panel">
      <div className="section-heading">
        <div>
          <h2>今日任务看板</h2>
          <p>{enabled ? `${board?.totalCount ?? 0} 个本地条目` : "已在设置中隐藏"}</p>
        </div>
      </div>
      {!enabled ? (
        <div className="empty-state">任务看板已关闭。</div>
      ) : !board ? (
        <div className="empty-state">暂无任务数据。</div>
      ) : (
        <div className="task-columns">
          {board.columns.map((column) => (
            <TaskColumnView column={column} key={column.id} />
          ))}
        </div>
      )}
    </section>
  );
}

function TaskColumnView({ column }: { column: TaskColumn }) {
  return (
    <div className={`task-column ${column.id}`}>
      <div className="task-column-header">
        {icons[column.id]}
        <span>{column.title}</span>
        <strong>{column.count}</strong>
      </div>
      <div className="task-list">
        {column.items.length === 0 ? (
          <p className="task-empty">无</p>
        ) : (
          column.items.map((item) => (
            <article className="task-card" key={item.id}>
              <div className="task-code">{item.code}</div>
              <h3 title={item.title}>{item.title}</h3>
              <p title={item.detail || "暂无详情"}>{item.detail || "暂无详情"}</p>
              <span>{item.chip}</span>
            </article>
          ))
        )}
      </div>
    </div>
  );
}
