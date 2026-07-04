import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import {
  createCodexConfigBackup,
  deleteCodexConfigBackup,
  getDetectionPaths,
  listCodexConfigBackups,
  restoreCodexConfigBackup,
} from "../lib/api";
import { defaultSettings } from "../lib/mock";
import { SettingsDrawer } from "./SettingsDrawer";

vi.mock("../lib/api", () => ({
  createCodexConfigBackup: vi.fn(),
  deleteCodexConfigBackup: vi.fn(),
  getDetectionPaths: vi.fn(),
  listCodexConfigBackups: vi.fn(),
  restoreCodexConfigBackup: vi.fn(),
}));

const defaultBackup = {
  id: "default-initial",
  label: "首次启动默认配置",
  createdAt: "2026-07-04T12:00:00.000Z",
  isDefault: true,
  hasConfig: true,
  hasAuth: true,
};

const manualBackup = {
  id: "manual-20260704120000123",
  label: "手动备份",
  createdAt: "2026-07-04T12:05:00.000Z",
  isDefault: false,
  hasConfig: true,
  hasAuth: true,
};

describe("SettingsDrawer", () => {
  beforeEach(() => {
    vi.mocked(getDetectionPaths).mockResolvedValue({
      codexBinaryPath: "codex",
      codexDataDir: "~/.codex",
      stateDbPath: "~/.codex/state_5.sqlite",
      appLogDir: "logs",
    });
    vi.mocked(listCodexConfigBackups).mockResolvedValue([defaultBackup]);
    vi.mocked(createCodexConfigBackup).mockResolvedValue([defaultBackup, manualBackup]);
    vi.mocked(restoreCodexConfigBackup).mockResolvedValue([defaultBackup, manualBackup]);
    vi.mocked(deleteCodexConfigBackup).mockResolvedValue([defaultBackup]);
  });

  it("hides relay-only fields in official mode", () => {
    render(<SettingsDrawer settings={defaultSettings} onClose={() => {}} onSave={() => {}} />);

    expect(screen.getByText("设置")).toBeInTheDocument();
    expect(
      screen.queryByText("接入方式、路径、刷新频率、主题与任务看板行为"),
    ).not.toBeInTheDocument();
    expect(screen.queryByLabelText("API 地址")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("模型名字")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("推理强度")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("速度策略")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "日志" })).not.toBeInTheDocument();
  });

  it("shows config backup controls above access mode", async () => {
    render(<SettingsDrawer settings={defaultSettings} onClose={() => {}} onSave={() => {}} />);

    expect(await screen.findByLabelText("配置备份")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /保存备份/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /恢复备份/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /删除备份/ })).toBeDisabled();
    expect(screen.getByLabelText("配置备份").compareDocumentPosition(screen.getByLabelText("当前模式"))).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING,
    );
  });

  it("deletes the selected manual config backup", async () => {
    vi.mocked(listCodexConfigBackups).mockResolvedValue([defaultBackup, manualBackup]);
    render(<SettingsDrawer settings={defaultSettings} onClose={() => {}} onSave={() => {}} />);

    const backupSelect = (await screen.findByLabelText("配置备份")) as HTMLSelectElement;
    fireEvent.change(backupSelect, { target: { value: manualBackup.id } });
    fireEvent.click(screen.getByRole("button", { name: /删除备份/ }));

    await waitFor(() => expect(deleteCodexConfigBackup).toHaveBeenCalledWith(manualBackup.id));
    expect(await screen.findByText("已删除所选配置备份")).toBeInTheDocument();
  });

  it("normalizes relay API endpoint to a single v1 suffix and closes after save", async () => {
    const onClose = vi.fn();
    const onSave = vi.fn().mockResolvedValue(undefined);
    render(<SettingsDrawer settings={defaultSettings} onClose={onClose} onSave={onSave} />);

    fireEvent.change(screen.getByLabelText("当前模式"), { target: { value: "relay" } });
    const endpoint = screen.getByLabelText("API 地址") as HTMLInputElement;
    fireEvent.change(endpoint, { target: { value: "api.example.com/v1/v1/" } });
    fireEvent.blur(endpoint);

    expect(endpoint).toHaveValue("https://api.example.com/v1");

    fireEvent.click(screen.getByRole("button", { name: "保存设置" }));
    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        accessMode: "relay",
        apiEndpoint: "https://api.example.com/v1",
      }),
    );
    await waitFor(() => expect(onClose).toHaveBeenCalled());
  });

  it("clears relay fields when switching back to official mode", async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    render(
      <SettingsDrawer
        settings={{
          ...defaultSettings,
          accessMode: "relay",
          apiEndpoint: "https://api.example.com/v1",
          apiKey: "sk-test",
          apiModel: "relay-model",
          reasoningEffort: "extreme",
          speedMode: "fast",
        }}
        onClose={() => {}}
        onSave={onSave}
      />,
    );

    fireEvent.change(screen.getByLabelText("当前模式"), { target: { value: "official" } });
    fireEvent.click(screen.getByRole("button", { name: "保存设置" }));

    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        accessMode: "official",
        apiEndpoint: null,
        apiKey: null,
        apiModel: "gpt-5",
        reasoningEffort: "medium",
        speedMode: "balanced",
      }),
    );
  });
});
