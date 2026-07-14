import { render, screen } from "@testing-library/react";
import { QuotaPanel } from "./QuotaPanel";
import { mockSnapshot } from "../lib/mock";

describe("QuotaPanel", () => {
  it("renders a complete single-ring seven-day official quota card", () => {
    const { container } = render(<QuotaPanel snapshot={mockSnapshot} isLoading={false} />);
    expect(screen.getByText("额度窗口")).toBeInTheDocument();
    expect(screen.queryByText("5 小时本地 token 用量占比")).not.toBeInTheDocument();
    expect(screen.getByText("官方 7 天额度")).toBeInTheDocument();
    expect(screen.getByText("7 天滚动窗口")).toBeInTheDocument();
    expect(screen.getByText("已用 58%")).toBeInTheDocument();
    expect(container.querySelector(".dual-ring")).not.toBeInTheDocument();
    expect(container.querySelectorAll(".quota-ring .ring-track")).toHaveLength(1);
    expect(container.querySelectorAll(".quota-ring .ring-value")).toHaveLength(1);
  });

  it("hides official quota windows in API relay mode", () => {
    render(<QuotaPanel snapshot={mockSnapshot} isLoading={false} accessMode="relay" />);
    expect(screen.getByText("API 模式使用本地会话统计")).toBeInTheDocument();
    expect(screen.queryByText("5 小时本地 token 用量占比")).not.toBeInTheDocument();
    expect(screen.queryByText("7 天剩余")).not.toBeInTheDocument();
  });

  it("does not render local five-hour usage when seven-day quota is available", () => {
    render(
      <QuotaPanel
        snapshot={{
          ...mockSnapshot,
          primary: null,
          secondary: { usedPercent: 2, windowDurationMins: 10080, resetsAt: 1_784_542_018 },
          local: {
            ...mockSnapshot.local!,
            detailedUsage: {
              ...mockSnapshot.local!.detailedUsage!,
              fiveHourLocal: {
                estimatedCostUsd: 0.12,
                tokens: {
                  inputTokens: 12_000,
                  cachedInputTokens: 3_000,
                  outputTokens: 1_000,
                  reasoningOutputTokens: 0,
                  totalTokens: 13_000,
                },
              },
              sevenDay: {
                ...mockSnapshot.local!.detailedUsage!.sevenDay,
                tokens: {
                  ...mockSnapshot.local!.detailedUsage!.sevenDay.tokens,
                  totalTokens: 100_000,
                },
              },
            },
          },
        }}
        isLoading={false}
      />,
    );

    expect(screen.queryByText("5 小时本地 token 用量占比")).not.toBeInTheDocument();
    expect(screen.queryByText(/占近 7 天本地用量/)).not.toBeInTheDocument();
    expect(screen.queryByText("13%")).not.toBeInTheDocument();
    expect(screen.getAllByText("98%").length).toBeGreaterThan(0);
    expect(screen.getByText("已用 2%")).toBeInTheDocument();
  });
});
