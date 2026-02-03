/**
 * @file ProjectFilter.tsx
 * @description 项目筛选组件，用于侧边栏筛选话题
 * @module components/projects/ProjectFilter
 * @requirements 3.1
 */

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useProjects } from "@/hooks/useProjects";
import { FolderIcon, LayersIcon } from "lucide-react";

export interface ProjectFilterProps {
  /** 当前选中的项目 ID，null 表示全部项目 */
  value: string | null;
  /** 选择变化回调 */
  onChange: (projectId: string | null) => void;
  /** 自定义类名 */
  className?: string;
}

/**
 * 项目筛选组件
 *
 * 显示项目筛选下拉框，支持选择"全部项目"或特定项目。
 */
export function ProjectFilter({
  value,
  onChange,
  className,
}: ProjectFilterProps) {
  const { projects, loading } = useProjects();

  // 过滤掉已归档的项目
  const availableProjects = projects.filter((p) => !p.isArchived);

  const handleChange = (val: string) => {
    onChange(val === "all" ? null : val);
  };

  return (
    <Select
      value={value || "all"}
      onValueChange={handleChange}
      disabled={loading}
    >
      <SelectTrigger className={className}>
        <SelectValue placeholder="全部项目" />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="all">
          <div className="flex items-center gap-2">
            <LayersIcon className="h-4 w-4 text-muted-foreground" />
            <span>全部项目</span>
          </div>
        </SelectItem>
        {availableProjects.map((project) => (
          <SelectItem key={project.id} value={project.id}>
            <div className="flex items-center gap-2">
              {project.icon ? (
                <span className="text-base">{project.icon}</span>
              ) : (
                <FolderIcon className="h-4 w-4 text-muted-foreground" />
              )}
              <span>{project.name}</span>
            </div>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

export default ProjectFilter;
