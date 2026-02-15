import { describe, expect, it } from "vitest";
import type { Project } from "@/types/project";
import { getAvailableProjects } from "./projectSelectorUtils";

function createProject(overrides: Partial<Project>): Project {
  return {
    id: "project-id",
    name: "项目",
    workspaceType: "general",
    rootPath: "/tmp/project",
    isDefault: false,
    icon: undefined,
    color: undefined,
    isFavorite: false,
    isArchived: false,
    tags: [],
    createdAt: 1,
    updatedAt: 1,
    ...overrides,
  };
}

describe("getAvailableProjects", () => {
  it("workspaceType 为 general 时只保留默认项目和通用项目", () => {
    const projects = [
      createProject({
        id: "default",
        name: "默认项目",
        isDefault: true,
        workspaceType: "general",
      }),
      createProject({
        id: "general-1",
        name: "通用项目",
        workspaceType: "general",
      }),
      createProject({
        id: "social-1",
        name: "社媒项目",
        workspaceType: "social-media",
      }),
    ];

    const result = getAvailableProjects(projects, "general");

    expect(result.map((project) => project.id)).toEqual(["default", "general-1"]);
  });

  it("workspaceType 为其他类型时保留该类型和默认项目，并排除归档", () => {
    const projects = [
      createProject({
        id: "default",
        name: "默认项目",
        isDefault: true,
        workspaceType: "general",
      }),
      createProject({
        id: "social-1",
        name: "社媒项目 A",
        workspaceType: "social-media",
      }),
      createProject({
        id: "social-archived",
        name: "社媒项目归档",
        workspaceType: "social-media",
        isArchived: true,
      }),
      createProject({
        id: "general-1",
        name: "通用项目",
        workspaceType: "general",
      }),
    ];

    const result = getAvailableProjects(projects, "social-media");

    expect(result.map((project) => project.id)).toEqual(["default", "social-1"]);
  });

  it("未提供 workspaceType 时返回全部未归档项目，默认项目置顶", () => {
    const projects = [
      createProject({
        id: "social-1",
        name: "社媒项目",
        workspaceType: "social-media",
      }),
      createProject({
        id: "default",
        name: "默认项目",
        isDefault: true,
        workspaceType: "general",
      }),
      createProject({
        id: "general-1",
        name: "通用项目",
        workspaceType: "general",
      }),
      createProject({
        id: "archived",
        name: "归档项目",
        workspaceType: "general",
        isArchived: true,
      }),
    ];

    const result = getAvailableProjects(projects);

    expect(result.map((project) => project.id)).toEqual([
      "default",
      "social-1",
      "general-1",
    ]);
  });
});
