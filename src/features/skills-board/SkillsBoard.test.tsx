import { fireEvent, render, screen } from "@testing-library/react";
import * as skillsApi from "./api";
import { SkillsBoard } from "./SkillsBoard";
import type { SkillBoardData } from "./types";

vi.mock("./api", () => ({
  getSkillBoard: vi.fn(async (): Promise<SkillBoardData> => ({
    refreshedAt: "2026-07-04 15:00:00",
    totalCount: 3,
    messages: [],
    skills: [
      {
        id: "user:demo",
        name: "demo",
        description: "只显示技能描述",
        sourceKind: "user",
        sourceLabel: "用户技能",
        status: "enabled",
        folderPath: "C:\\Users\\a3960\\.codex\\skills\\demo",
        skillFilePath: "C:\\Users\\a3960\\.codex\\skills\\demo\\SKILL.md",
        canEnable: false,
        canDisable: true,
        canDelete: true,
        canOpenFolder: true,
      },
      {
        id: "plugin:demo",
        name: "plugin-demo",
        description: "插件只读",
        sourceKind: "plugin",
        sourceLabel: "插件技能",
        status: "readOnly",
        folderPath: "C:\\Users\\a3960\\.codex\\plugins\\cache\\demo",
        skillFilePath: "C:\\Users\\a3960\\.codex\\plugins\\cache\\demo\\SKILL.md",
        canEnable: false,
        canDisable: false,
        canDelete: false,
        canOpenFolder: true,
      },
      {
        id: "disabled:old-demo",
        name: "old-demo",
        description: "Disabled skill",
        sourceKind: "disabled",
        sourceLabel: "已禁用",
        status: "disabled",
        folderPath: "C:\\Users\\a3960\\.codex\\skills-disabled\\old-demo",
        skillFilePath: "C:\\Users\\a3960\\.codex\\skills-disabled\\old-demo\\SKILL.md",
        canEnable: true,
        canDisable: false,
        canDelete: false,
        canOpenFolder: true,
      },
    ],
  })),
  disableSkill: vi.fn(async (): Promise<SkillBoardData> => ({
    refreshedAt: "2026-07-04 15:01:00",
    totalCount: 3,
    messages: [],
    skills: [
      {
        id: "disabled:demo",
        name: "demo",
        description: "Disabled now",
        sourceKind: "disabled",
        sourceLabel: "已禁用",
        status: "disabled",
        folderPath: "/Users/mac/.codex/skills-disabled/demo",
        skillFilePath: "/Users/mac/.codex/skills-disabled/demo/SKILL.md",
        canEnable: true,
        canDisable: false,
        canDelete: false,
        canOpenFolder: true,
      },
    ],
  })),
  enableSkill: vi.fn(async (): Promise<SkillBoardData> => ({
    refreshedAt: "2026-07-04 15:01:00",
    totalCount: 1,
    messages: [],
    skills: [
      {
        id: "user:old-demo",
        name: "old-demo",
        description: "Enabled again",
        sourceKind: "user",
        sourceLabel: "用户技能",
        status: "enabled",
        folderPath: "C:\\Users\\a3960\\.codex\\skills\\old-demo",
        skillFilePath: "C:\\Users\\a3960\\.codex\\skills\\old-demo\\SKILL.md",
        canEnable: false,
        canDisable: true,
        canDelete: true,
        canOpenFolder: true,
      },
    ],
  })),
  archiveSkill: vi.fn(),
  openSkillFolder: vi.fn(),
  translateDescriptionToChinese: vi.fn(async () => "Translated description"),
}));

describe("SkillsBoard", () => {
  it("renders skill names and selected description", async () => {
    render(<SkillsBoard enabled />);

    expect(await screen.findByText("Skills 技能看板")).toBeInTheDocument();
    const options = await screen.findAllByRole("option");
    expect(options.some((option) => option.textContent?.includes("demo"))).toBe(true);
    expect(screen.getByText("只显示技能描述")).toBeInTheDocument();
    expect(screen.getByLabelText("禁用 demo")).toHaveClass("state-enabled");
    expect(screen.getByLabelText("禁用 plugin-demo")).toBeDisabled();
    expect(screen.getByLabelText("删除 plugin-demo")).toBeDisabled();
  });

  it("filters disabled skills and toggles translated description", async () => {
    render(<SkillsBoard enabled />);

    expect(await screen.findByText("Skills 技能看板")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /已禁用/ }));
    expect(screen.getByRole("option", { name: /old-demo/ })).toBeInTheDocument();
    expect(screen.getByLabelText("启用 old-demo")).toHaveClass("state-disabled");
    expect(screen.queryByRole("option", { name: /^demo/ })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "翻译技能描述" }));
    expect(await screen.findByText("Translated description")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "取消翻译" }));
    expect(screen.getByText("Disabled skill")).toBeInTheDocument();
  });

  it("enables a disabled skill from the disabled filter", async () => {
    render(<SkillsBoard enabled />);

    expect(await screen.findByText("Skills 技能看板")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /已禁用/ }));
    fireEvent.click(screen.getByLabelText("启用 old-demo"));

    expect(skillsApi.enableSkill).toHaveBeenCalledWith("disabled:old-demo");
    expect(await screen.findByText("Enabled again")).toBeInTheDocument();
  });

  it("disables an enabled skill without a blocking confirm dialog", async () => {
    const confirmSpy = vi.spyOn(window, "confirm");
    render(<SkillsBoard enabled />);

    expect(await screen.findByText("Skills 技能看板")).toBeInTheDocument();
    fireEvent.click(screen.getByLabelText("禁用 demo"));

    expect(confirmSpy).not.toHaveBeenCalled();
    expect(skillsApi.disableSkill).toHaveBeenCalledWith("user:demo");
    expect(screen.getByRole("button", { name: /全部/ })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: /已禁用/ })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
    expect(await screen.findByText("Disabled now")).toBeInTheDocument();
    expect(screen.getByLabelText("启用 demo")).toHaveClass("state-disabled");
  });
});
