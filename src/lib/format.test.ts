import { formatTokens, remainingPercent, splitTokens } from "./format";

describe("format helpers", () => {
  it("formats token counts compactly", () => {
    expect(formatTokens(1_250_000)).toBe("125万");
    expect(formatTokens(12_300)).toBe("1.2万");
  });

  it("calculates remaining quota", () => {
    expect(remainingPercent({ usedPercent: 42 })).toBe(58);
  });

  it("splits cached and uncached input", () => {
    const split = splitTokens({
      inputTokens: 100,
      cachedInputTokens: 40,
      outputTokens: 25,
      reasoningOutputTokens: 0,
      totalTokens: 125,
    });
    expect(split.uncached).toBe(60);
    expect(split.cached).toBe(40);
    expect(split.output).toBe(25);
  });
});
