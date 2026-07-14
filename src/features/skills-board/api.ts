import { invoke } from "@tauri-apps/api/core";
import type { SkillBoardData } from "./types";
export { translateToChinese as translateDescriptionToChinese } from "../../lib/translation";

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
