import { render, screen } from "@testing-library/react";
import { QuotaPanel } from "./QuotaPanel";
import { mockSnapshot } from "../lib/mock";

describe("QuotaPanel", () => {
  it("renders remaining quota labels", () => {
    render(<QuotaPanel snapshot={mockSnapshot} isLoading={false} />);
    expect(screen.getByText("额度窗口")).toBeInTheDocument();
    expect(screen.getByText("5 小时剩余")).toBeInTheDocument();
    expect(screen.getByText("7 天剩余")).toBeInTheDocument();
  });

  it("hides official quota windows in API relay mode", () => {
    render(<QuotaPanel snapshot={mockSnapshot} isLoading={false} accessMode="relay" />);
    expect(screen.getByText("API 模式使用本地会话统计")).toBeInTheDocument();
    expect(screen.queryByText("5 小时剩余")).not.toBeInTheDocument();
    expect(screen.queryByText("7 天剩余")).not.toBeInTheDocument();
  });
});
