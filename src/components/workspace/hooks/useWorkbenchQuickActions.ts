import { useMemo } from "react";
import { Bot, FileText, Send, Sparkles, Wrench } from "lucide-react";
import type { ThemeWorkspaceView } from "@/features/themes";
import type { WorkspaceViewMode } from "@/types/page";
import type { WorkbenchQuickAction } from "@/components/workspace/panels";

export interface UseWorkbenchQuickActionsParams {
  workspaceMode: WorkspaceViewMode;
  activeWorkspaceView: ThemeWorkspaceView;
  hasWorkflowWorkspaceView: boolean;
  hasPublishWorkspaceView: boolean;
  hasSettingsWorkspaceView: boolean;
  selectedContentId: string | null;
  onSwitchWorkspaceView: (view: ThemeWorkspaceView) => void;
  onQuickSaveCurrent: () => Promise<void> | void;
}

export function useWorkbenchQuickActions({
  workspaceMode,
  activeWorkspaceView,
  hasWorkflowWorkspaceView,
  hasPublishWorkspaceView,
  hasSettingsWorkspaceView,
  selectedContentId,
  onSwitchWorkspaceView,
  onQuickSaveCurrent,
}: UseWorkbenchQuickActionsParams) {
  const nonCreateQuickActions = useMemo<WorkbenchQuickAction[]>(() => {
    if (workspaceMode !== "workspace" || activeWorkspaceView === "create") {
      return [];
    }

    const actions: WorkbenchQuickAction[] = [
      {
        key: "to-create",
        label: "返回创作视图",
        icon: Bot,
        onClick: () => onSwitchWorkspaceView("create"),
      },
    ];

    if (hasWorkflowWorkspaceView && activeWorkspaceView !== "workflow") {
      actions.push({
        key: "to-workflow",
        label: "前往流程视图",
        icon: Sparkles,
        onClick: () => onSwitchWorkspaceView("workflow"),
      });
    }

    if (hasPublishWorkspaceView && activeWorkspaceView !== "publish") {
      actions.push({
        key: "to-publish",
        label: "前往发布视图",
        icon: Send,
        onClick: () => onSwitchWorkspaceView("publish"),
      });
    }

    if (hasSettingsWorkspaceView && activeWorkspaceView !== "settings") {
      actions.push({
        key: "to-settings",
        label: "前往设置视图",
        icon: Wrench,
        onClick: () => onSwitchWorkspaceView("settings"),
      });
    }

    if (selectedContentId) {
      actions.push({
        key: "quick-save",
        label: "快速保存当前文稿",
        icon: FileText,
        onClick: () => {
          void onQuickSaveCurrent();
        },
      });
    }

    return actions;
  }, [
    activeWorkspaceView,
    hasPublishWorkspaceView,
    hasSettingsWorkspaceView,
    hasWorkflowWorkspaceView,
    onQuickSaveCurrent,
    onSwitchWorkspaceView,
    selectedContentId,
    workspaceMode,
  ]);

  return {
    nonCreateQuickActions,
  };
}

export default useWorkbenchQuickActions;
