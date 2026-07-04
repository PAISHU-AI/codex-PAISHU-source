export type SkillSourceKind = "user" | "system" | "plugin" | "disabled";

export type SkillStatus = "enabled" | "disabled" | "readOnly";

export interface SkillSummary {
  id: string;
  name: string;
  description: string;
  sourceKind: SkillSourceKind;
  sourceLabel: string;
  status: SkillStatus;
  folderPath: string;
  skillFilePath: string;
  canEnable: boolean;
  canDisable: boolean;
  canDelete: boolean;
  canOpenFolder: boolean;
}

export interface SkillBoardData {
  refreshedAt: string;
  totalCount: number;
  skills: SkillSummary[];
  messages: string[];
}
