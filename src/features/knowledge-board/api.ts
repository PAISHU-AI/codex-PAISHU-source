import { invoke } from "@tauri-apps/api/core";
import { translateToChinese } from "../../lib/translation";
import type { KnowledgeBoardData, KnowledgeOverview } from "./types";

export function getKnowledgeBoard(): Promise<KnowledgeBoardData> {
  return invoke<KnowledgeBoardData>("get_knowledge_board");
}

export function getKnowledgeOverview(documentId: string): Promise<KnowledgeOverview> {
  return invoke<KnowledgeOverview>("get_knowledge_overview", { documentId });
}

export function translateKnowledgeOverviewToChinese(text: string): Promise<string> {
  return translateToChinese(text);
}

export function setKnowledgeEnabled(
  documentId: string,
  enabled: boolean,
): Promise<KnowledgeBoardData> {
  return invoke<KnowledgeBoardData>("set_knowledge_enabled", { documentId, enabled });
}
