import type { Project } from "@/types/project";

/**
 * 计算可选项目列表
 *
 * 规则：
 * 1. 始终排除归档项目
 * 2. 提供 workspaceType 时仅显示该类型项目 + 默认项目
 * 3. 默认项目固定置顶
 */
export function getAvailableProjects(
  projects: Project[],
  workspaceType?: string,
): Project[] {
  let filtered = projects.filter((project) => !project.isArchived);

  if (workspaceType) {
    filtered = filtered.filter(
      (project) => project.isDefault || project.workspaceType === workspaceType,
    );
  }

  return [...filtered].sort((a, b) => {
    if (a.isDefault && !b.isDefault) return -1;
    if (!a.isDefault && b.isDefault) return 1;
    return 0;
  });
}
