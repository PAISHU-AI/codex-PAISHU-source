import { render, screen } from "@testing-library/react";
import { TaskBoard } from "./TaskBoard";
import { mockSnapshot } from "../lib/mock";

describe("TaskBoard", () => {
  it("renders task columns and cards", () => {
    render(<TaskBoard board={mockSnapshot.taskBoard!} enabled />);
    expect(screen.getByText("今日任务看板")).toBeInTheDocument();
    expect(screen.getByText("进行中")).toBeInTheDocument();
    expect(screen.getByText("重构光核超级服务桌面工作台")).toBeInTheDocument();
  });
});
