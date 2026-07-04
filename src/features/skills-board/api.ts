import { invoke } from "@tauri-apps/api/core";
import type { SkillBoardData } from "./types";

export function getSkillBoard(): Promise<SkillBoardData> {
  return invoke<SkillBoardData>("get_skill_board");
}

export function disableSkill(skillId: string): Promise<SkillBoardData> {
  return invoke<SkillBoardData>("disable_skill", { skillId });
}

export function enableSkill(skillId: string): Promise<SkillBoardData> {
  return invoke<SkillBoardData>("enable_skill", { skillId });
}

export function archiveSkill(skillId: string): Promise<SkillBoardData> {
  return invoke<SkillBoardData>("archive_skill", { skillId });
}

export function openSkillFolder(skillId: string): Promise<string> {
  return invoke<string>("open_skill_folder", { skillId });
}

export async function translateDescriptionToChinese(text: string): Promise<string> {
  const trimmed = text.trim();
  if (!trimmed) return "";
  const url = `https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=zh-CN&dt=t&q=${encodeURIComponent(trimmed)}`;
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Google 翻译请求失败：${response.status}`);
  }
  const payload = (await response.json()) as unknown;
  const translated = extractGoogleTranslateText(payload);
  if (!translated) {
    throw new Error("Google 翻译没有返回可用文本");
  }
  return translated;
}

function extractGoogleTranslateText(payload: unknown): string {
  if (!Array.isArray(payload) || !Array.isArray(payload[0])) {
    return "";
  }
  return payload[0]
    .map((item) => {
      if (!Array.isArray(item)) return "";
      const text = item[0];
      return typeof text === "string" ? text : "";
    })
    .join("")
    .trim();
}
