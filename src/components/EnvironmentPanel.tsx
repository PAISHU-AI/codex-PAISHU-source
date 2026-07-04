import { AlertTriangle, Check, Info } from "lucide-react";
import type { DiagnosticItem } from "../types/usage";

interface EnvironmentPanelProps {
  diagnostics: DiagnosticItem[];
  isPartial: boolean;
}

export function EnvironmentPanel({ diagnostics, isPartial }: EnvironmentPanelProps) {
  return (
    <section className="panel environment-panel">
      <div className="section-heading">
        <div>
          <h2>环境诊断</h2>
          <p>{isPartial ? "部分数据模式" : "关键数据源就绪"}</p>
        </div>
        <Info size={18} />
      </div>
      <div className="diagnostic-list">
        {diagnostics.length === 0 ? (
          <div className="empty-state">刷新后显示诊断结果。</div>
        ) : (
          diagnostics.map((item) => (
            <div className={`diagnostic-item ${item.status}`} key={item.id}>
              {item.status === "ok" ? <Check size={15} /> : <AlertTriangle size={15} />}
              <div>
                <strong title={item.title}>{item.title}</strong>
                <p title={item.detail}>{item.detail}</p>
              </div>
            </div>
          ))
        )}
      </div>
    </section>
  );
}
