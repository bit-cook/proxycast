/**
 * @file ProjectSelector.tsx
 * @description 项目选择器组件，用于在聊天入口和侧边栏选择项目
 * @module components/projects/ProjectSelector
 * @requirements 4.1, 4.2, 4.3, 4.5
 */

import { useState, useEffect, useMemo } from "react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useProjects } from "@/hooks/useProjects";
import { getAvailableProjects } from "./projectSelectorUtils";
import { FolderIcon, StarIcon } from "lucide-react";

export interface ProjectSelectorProps {
  /** 当前选中的项目 ID */
  value: string | null;
  /** 选择变化回调 */
  onChange: (projectId: string) => void;
  /** 按主题类型筛选（可选，不传则显示所有项目） */
  workspaceType?: string;
  /** 占位符文本 */
  placeholder?: string;
  /** 是否禁用 */
  disabled?: boolean;
  /** 自定义类名 */
  className?: string;
  /** 下拉方向 */
  dropdownSide?: "top" | "bottom";
  /** 下拉对齐 */
  dropdownAlign?: "start" | "end";
}

/**
 * 项目选择器组件
 *
 * 显示项目下拉选择器，默认选中"默认项目"。
 */
export function ProjectSelector({
  value,
  onChange,
  workspaceType,
  placeholder = "选择项目",
  disabled = false,
  className,
  dropdownSide = "top",
  dropdownAlign = "start",
}: ProjectSelectorProps) {
  const { projects, defaultProject, loading, getOrCreateDefault } =
    useProjects();
  const [initialized, setInitialized] = useState(false);

  // 过滤项目：排除归档 + 按主题类型筛选
  const availableProjects = useMemo(() => {
    return getAvailableProjects(projects, workspaceType);
  }, [projects, workspaceType]);

  // 查找当前选中的项目
  const selectedProject = useMemo(
    () => availableProjects.find((p) => p.id === value),
    [availableProjects, value],
  );

  // 初始化时确保有默认项目
  useEffect(() => {
    if (!initialized && !loading) {
      if (!value && defaultProject) {
        onChange(defaultProject.id);
      } else if (!value && !defaultProject) {
        // 创建默认项目
        getOrCreateDefault().then((project) => {
          onChange(project.id);
        });
      }
      setInitialized(true);
    }
  }, [
    initialized,
    loading,
    value,
    defaultProject,
    onChange,
    getOrCreateDefault,
  ]);

  return (
    <Select
      value={value || undefined}
      onValueChange={onChange}
      disabled={disabled || loading}
    >
      <SelectTrigger className={className}>
        {/* 自定义显示选中项 */}
        {selectedProject ? (
          <div className="flex items-center gap-2">
            {selectedProject.icon ? (
              <span className="text-base">{selectedProject.icon}</span>
            ) : (
              <FolderIcon className="h-4 w-4 text-muted-foreground" />
            )}
            <span className="truncate">{selectedProject.name}</span>
            {selectedProject.isDefault && (
              <StarIcon className="h-3 w-3 text-yellow-500 fill-yellow-500 flex-shrink-0" />
            )}
          </div>
        ) : (
          <SelectValue placeholder={placeholder} />
        )}
      </SelectTrigger>
      <SelectContent
        side={dropdownSide}
        align={dropdownAlign}
        className="min-w-[320px] max-w-[520px]"
      >
        {availableProjects.map((project) => (
          <SelectItem key={project.id} value={project.id}>
            <div className="flex items-center gap-2 w-full">
              {project.icon ? (
                <span className="text-base flex-shrink-0">{project.icon}</span>
              ) : (
                <FolderIcon className="h-4 w-4 text-muted-foreground flex-shrink-0" />
              )}
              <span className="break-all">{project.name}</span>
              {project.isDefault && (
                <StarIcon className="h-3 w-3 text-yellow-500 fill-yellow-500 flex-shrink-0" />
              )}
            </div>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

export default ProjectSelector;
