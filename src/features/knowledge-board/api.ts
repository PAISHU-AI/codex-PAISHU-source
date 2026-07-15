import { invoke } from "@tauri-apps/api/core";
import { translateToChinese } from "../../lib/translation";
import type { KnowledgeBoardData, KnowledgeOverview } from "./types";

export function getKnowledgeBoard(): Promise<KnowledgeBoardData> {
  return invoke<KnowledgeBoardData>("get_knowledge_board");
}

export function syncKnowledgeSources(): Promise<KnowledgeBoardData> {
  return invoke<KnowledgeBoardData>("sync_knowledge_sources");
}

export function openKnowledgeSource(documentId: string): Promise<string> {
  return invoke<string>("open_knowledge_source", { documentId });
}

export function deleteKnowledge(documentId: string): Promise<KnowledgeBoardData> {
  return invoke<KnowledgeBoardData>("delete_knowledge", { documentId });
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
