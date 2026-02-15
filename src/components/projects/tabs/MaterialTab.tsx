/**
 * @file MaterialTab.tsx
 * @description 素材 Tab 组件，管理项目素材
 * @module components/projects/tabs/MaterialTab
 * @requirements 7.1, 7.2, 7.3, 7.4, 7.5, 7.6
 */

import { useState } from "react";
import { useMaterials } from "@/hooks/useMaterials";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  PlusIcon,
  SearchIcon,
  FileIcon,
  ImageIcon,
  FileTextIcon,
  DatabaseIcon,
  LinkIcon,
  Music2Icon,
  VideoIcon,
  TrashIcon,
  EyeIcon,
  PaletteIcon,
  LayoutIcon,
} from "lucide-react";
import type {
  Material,
  MaterialType,
  UploadMaterialRequest,
} from "@/types/material";
import { MaterialTypeLabels } from "@/types/material";
import { MaterialUploadDialog, MaterialPreviewDialog } from "../dialogs";

export interface MaterialTabProps {
  /** 项目 ID */
  projectId: string;
}

const MaterialTypeIcons: Record<MaterialType, typeof FileIcon> = {
  document: FileIcon,
  image: ImageIcon,
  audio: Music2Icon,
  video: VideoIcon,
  text: FileTextIcon,
  data: DatabaseIcon,
  link: LinkIcon,
  icon: FileIcon,
  color: PaletteIcon,
  layout: LayoutIcon,
};

/**
 * 素材 Tab 组件
 *
 * 显示素材网格，支持上传、预览、删除和筛选。
 */
export function MaterialTab({ projectId }: MaterialTabProps) {
  const {
    filteredMaterials,
    count,
    loading,
    filter,
    setFilter,
    upload,
    remove,
  } = useMaterials(projectId);
  const [searchQuery, setSearchQuery] = useState("");
  const [uploadDialogOpen, setUploadDialogOpen] = useState(false);
  const [previewMaterial, setPreviewMaterial] = useState<Material | null>(null);

  const handleSearch = (query: string) => {
    setSearchQuery(query);
    setFilter({ ...filter, searchQuery: query });
  };

  const handleUpload = async (data: UploadMaterialRequest) => {
    await upload(data);
  };

  const handleTypeFilter = (type: MaterialType | null) => {
    setFilter({ ...filter, type: type || undefined });
  };

  const handleDelete = async (id: string) => {
    if (confirm("确定要删除这个素材吗？此操作不可恢复。")) {
      await remove(id);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted-foreground">加载中...</div>
      </div>
    );
  }

  return (
    <div className="p-4 space-y-4">
      {/* 头部操作栏 */}
      <div className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-2 flex-1">
          <div className="relative flex-1 max-w-sm">
            <SearchIcon className="absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="搜索素材..."
              value={searchQuery}
              onChange={(e) => handleSearch(e.target.value)}
              className="pl-8"
            />
          </div>
          <div className="flex gap-1">
            <Button
              variant={!filter.type ? "secondary" : "ghost"}
              size="sm"
              onClick={() => handleTypeFilter(null)}
            >
              全部
            </Button>
            {(Object.keys(MaterialTypeLabels) as MaterialType[]).map((type) => (
              <Button
                key={type}
                variant={filter.type === type ? "secondary" : "ghost"}
                size="sm"
                onClick={() => handleTypeFilter(type)}
              >
                {MaterialTypeLabels[type]}
              </Button>
            ))}
          </div>
        </div>
        <Button onClick={() => setUploadDialogOpen(true)}>
          <PlusIcon className="h-4 w-4 mr-1" />
          上传素材
        </Button>
      </div>

      {/* 素材统计 */}
      <div className="text-sm text-muted-foreground">
        共 {count} 个素材
        {filter.type && `，筛选: ${MaterialTypeLabels[filter.type]}`}
        {searchQuery && `，搜索: "${searchQuery}"`}
      </div>

      {/* 素材网格 */}
      {filteredMaterials.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
          <FileIcon className="h-12 w-12 mb-4 opacity-50" />
          <p className="text-lg mb-2">暂无素材</p>
          <p className="text-sm mb-4">上传素材供 AI 创作时引用</p>
          <Button variant="outline" onClick={() => setUploadDialogOpen(true)}>
            <PlusIcon className="h-4 w-4 mr-1" />
            上传素材
          </Button>
        </div>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
          {filteredMaterials.map((material) => {
            const Icon =
              MaterialTypeIcons[material.type as MaterialType] || FileIcon;
            return (
              <div
                key={material.id}
                className="p-4 rounded-lg border bg-card space-y-3"
              >
                {/* 图标和类型 */}
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Icon className="h-5 w-5 text-muted-foreground" />
                    <Badge variant="outline" className="text-xs">
                      {MaterialTypeLabels[material.type as MaterialType] ||
                        material.type}
                    </Badge>
                  </div>
                  <div className="flex items-center gap-1">
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7"
                      onClick={() => setPreviewMaterial(material)}
                    >
                      <EyeIcon className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7 text-destructive"
                      onClick={() => handleDelete(material.id)}
                    >
                      <TrashIcon className="h-4 w-4" />
                    </Button>
                  </div>
                </div>

                {/* 名称 */}
                <div className="font-medium truncate" title={material.name}>
                  {material.name}
                </div>

                {/* 描述 */}
                {material.description && (
                  <p className="text-sm text-muted-foreground line-clamp-2">
                    {material.description}
                  </p>
                )}

                {/* 标签 */}
                {material.tags.length > 0 && (
                  <div className="flex flex-wrap gap-1">
                    {material.tags.slice(0, 3).map((tag) => (
                      <Badge key={tag} variant="secondary" className="text-xs">
                        {tag}
                      </Badge>
                    ))}
                    {material.tags.length > 3 && (
                      <Badge variant="secondary" className="text-xs">
                        +{material.tags.length - 3}
                      </Badge>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}

      {/* 上传对话框 */}
      <MaterialUploadDialog
        open={uploadDialogOpen}
        onOpenChange={setUploadDialogOpen}
        projectId={projectId}
        onUpload={handleUpload}
      />

      {/* 预览对话框 */}
      <MaterialPreviewDialog
        open={!!previewMaterial}
        onOpenChange={(open) => !open && setPreviewMaterial(null)}
        material={previewMaterial}
      />
    </div>
  );
}

export default MaterialTab;
