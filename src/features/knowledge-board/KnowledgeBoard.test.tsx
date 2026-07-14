import { fireEvent, render, screen } from "@testing-library/react";
import * as knowledgeApi from "./api";
import { KnowledgeBoard } from "./KnowledgeBoard";
import type { KnowledgeBoardData } from "./types";

const enabledBoard: KnowledgeBoardData = {
  refreshedAt: "2026-07-15 09:00:00",
  serviceStatus: "ok",
  collectionCount: 1,
  totalDocuments: 49,
  enabledDocuments: 48,
  disabledDocuments: 1,
  chunkCount: 252,
  databaseBytes: 19_293_798,
  averageReadMs: 42,
  readSuccessCount: 128,
  readFailureCount: 3,
  messages: [],
  documents: [
    {
      id: "d50f8262-c19d-46cd-a001-5d634b692807",
      title: "客户素材需求清单",
      sourceUri: "/knowledge-retrieval/client-materials.md",
      owner: "PAISHU Knowledge Steward",
      status: "active",
      accessTier: "internal",
      enabled: true,
      chunkCount: 12,
      approximateTokens: 8_400,
      updatedAt: "2026-07-15T08:55:00+08:00",
    },
  ],
};

vi.mock("./api", () => ({
  getKnowledgeBoard: vi.fn(async () => enabledBoard),
  getKnowledgeOverview: vi.fn(async () => ({
    documentId: "d50f8262-c19d-46cd-a001-5d634b692807",
    title: "客户素材需求清单",
    language: "en",
    overview: "Customers should provide brand assets and product details.",
    sourceUri: "/knowledge-retrieval/client-materials.md",
    updatedAt: "2026-07-15T08:55:00+08:00",
  })),
  translateKnowledgeOverviewToChinese: vi.fn(async () => "客户应提供品牌资料与产品说明。"),
  setKnowledgeEnabled: vi.fn(async () => ({
    ...enabledBoard,
    enabledDocuments: 47,
    disabledDocuments: 2,
    documents: enabledBoard.documents.map((document) => ({ ...document, enabled: false })),
  })),
}));

describe("KnowledgeBoard", () => {
  it("shows vector knowledge metrics and inventory", async () => {
    render(<KnowledgeBoard />);

    expect(await screen.findByText("知识库可视化")).toBeInTheDocument();
    expect(screen.getByText("49 份知识")).toBeInTheDocument();
    expect(screen.getByText("252 个分块")).toBeInTheDocument();
    expect(screen.getByText("18.4 MB")).toBeInTheDocument();
    expect(screen.getByText("42 ms")).toBeInTheDocument();
    expect(screen.getByText("128")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getByRole("option", { name: /客户素材需求清单/ })).toBeInTheDocument();
  });

  it("disables an enabled knowledge document", async () => {
    render(<KnowledgeBoard />);

    expect(await screen.findByRole("option", { name: /客户素材需求清单/ })).toBeInTheDocument();
    fireEvent.click(screen.getByLabelText("禁用 客户素材需求清单"));

    expect(knowledgeApi.setKnowledgeEnabled).toHaveBeenCalledWith(
      "d50f8262-c19d-46cd-a001-5d634b692807",
      false,
    );
    expect(await screen.findByLabelText("启用 客户素材需求清单")).toBeInTheDocument();
  });

  it("shows a real knowledge overview and toggles its Chinese translation", async () => {
    render(<KnowledgeBoard />);

    expect(
      await screen.findByText("Customers should provide brand assets and product details."),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "翻译知识概述" }));
    expect(await screen.findByText("客户应提供品牌资料与产品说明。")).toBeInTheDocument();
    expect(knowledgeApi.translateKnowledgeOverviewToChinese).toHaveBeenCalledWith(
      "Customers should provide brand assets and product details.",
    );

    fireEvent.click(screen.getByRole("button", { name: "取消知识概述翻译" }));
    expect(
      await screen.findByText("Customers should provide brand assets and product details."),
    ).toBeInTheDocument();
  });
});
