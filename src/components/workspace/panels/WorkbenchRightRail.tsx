import { Bot, FileText, FolderOpen, Sparkles, Wrench, type LucideIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import type { WorkflowProgressSnapshot } from "@/components/agent/chat";

export interface WorkbenchQuickAction {
  key: string;
  label: string;
  icon: LucideIcon;
  onClick: () => void;
  disabled?: boolean;
}

export interface WorkbenchRightRailProps {
  shouldRender: boolean;
  isCreateWorkspaceView: boolean;
  activeRightDrawer: "tools" | null;
  showChatPanel: boolean;
  onToggleChatPanel: () => void;
  onToggleToolsDrawer: () => void;
  onCloseToolsDrawer: () => void;
  workflowProgress: WorkflowProgressSnapshot | null;
  showWorkflowRail: boolean;
  onToggleWorkflowRail: () => void;
  onQuickSaveCurrent: () => void;
  selectedContentId: string | null;
  onOpenWorkflowView: () => void;
  selectedProjectId: string | null;
  hasWorkflowWorkspaceView: boolean;
  activeWorkspaceViewLabel: string;
  currentContentTitle: string | null;
  nonCreateQuickActions: WorkbenchQuickAction[];
  onBackToCreateView: () => void;
  getWorkflowStepStatusLabel: (
    status: WorkflowProgressSnapshot["steps"][number]["status"],
  ) => string;
}

export function WorkbenchRightRail({
  shouldRender,
  isCreateWorkspaceView,
  activeRightDrawer,
  showChatPanel,
  onToggleChatPanel,
  onToggleToolsDrawer,
  onCloseToolsDrawer,
  workflowProgress,
  showWorkflowRail,
  onToggleWorkflowRail,
  onQuickSaveCurrent,
  selectedContentId,
  onOpenWorkflowView,
  selectedProjectId,
  hasWorkflowWorkspaceView,
  activeWorkspaceViewLabel,
  currentContentTitle,
  nonCreateQuickActions,
  onBackToCreateView,
  getWorkflowStepStatusLabel,
}: WorkbenchRightRailProps) {
  if (!shouldRender) {
    return null;
  }

  return (
    <>
      {activeRightDrawer === "tools" && (
        <aside className="w-[260px] min-w-[260px] border-l bg-muted/10 p-4 flex flex-col gap-3">
          {isCreateWorkspaceView ? (
            <>
              <h3 className="text-sm font-semibold">主题工具</h3>
              <Button
                variant="outline"
                className="justify-start"
                onClick={onQuickSaveCurrent}
                disabled={!selectedContentId}
              >
                <FileText className="h-4 w-4 mr-2" />
                快速保存
              </Button>
              <Button
                variant="outline"
                className="justify-start"
                onClick={onOpenWorkflowView}
                disabled={!selectedProjectId}
              >
                <FolderOpen className="h-4 w-4 mr-2" />
                {hasWorkflowWorkspaceView ? "流程视图" : "项目设置"}
              </Button>
            </>
          ) : (
            <>
              <h3 className="text-sm font-semibold">视图动作</h3>
              <p className="text-xs text-muted-foreground">当前：{activeWorkspaceViewLabel}</p>
              {currentContentTitle && (
                <p className="text-xs text-muted-foreground truncate">
                  当前文稿：{currentContentTitle}
                </p>
              )}
              <div className="space-y-2">
                {nonCreateQuickActions.length === 0 ? (
                  <p className="text-xs text-muted-foreground">当前暂无可用动作</p>
                ) : (
                  nonCreateQuickActions.map((action) => {
                    const ActionIcon = action.icon;
                    return (
                      <Button
                        key={action.key}
                        variant="outline"
                        className="justify-start"
                        onClick={() => {
                          action.onClick();
                          onCloseToolsDrawer();
                        }}
                        disabled={action.disabled}
                      >
                        <ActionIcon className="h-4 w-4 mr-2" />
                        {action.label}
                      </Button>
                    );
                  })
                )}
              </div>
            </>
          )}
        </aside>
      )}
      <aside className="w-14 min-w-14 border-l bg-background/95 flex flex-col items-center py-3 gap-2">
        <TooltipProvider>
          {isCreateWorkspaceView ? (
            <>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    className={cn(
                      "h-9 w-9",
                      showChatPanel && "bg-accent text-accent-foreground",
                    )}
                    onClick={onToggleChatPanel}
                    title={showChatPanel ? "隐藏 AI 对话" : "显示 AI 对话"}
                  >
                    <Bot className="h-4 w-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="left">
                  <p>{showChatPanel ? "隐藏 AI 对话" : "显示 AI 对话"}</p>
                </TooltipContent>
              </Tooltip>

              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    className={cn(
                      "h-9 w-9",
                      activeRightDrawer === "tools" &&
                        "bg-accent text-accent-foreground",
                    )}
                    onClick={onToggleToolsDrawer}
                    title={
                      activeRightDrawer === "tools" ? "收起主题工具" : "展开主题工具"
                    }
                  >
                    <Wrench className="h-4 w-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="left">
                  <p>
                    {activeRightDrawer === "tools" ? "收起主题工具" : "展开主题工具"}
                  </p>
                </TooltipContent>
              </Tooltip>

              {workflowProgress && workflowProgress.steps.length > 0 && (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon"
                      className={cn(
                        "h-9 w-9",
                        showWorkflowRail && "bg-accent text-accent-foreground",
                      )}
                      onClick={onToggleWorkflowRail}
                      title={showWorkflowRail ? "收起流程步骤" : "展开流程步骤"}
                    >
                      <Sparkles className="h-4 w-4" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent side="left">
                    <p>{showWorkflowRail ? "收起流程步骤" : "展开流程步骤"}</p>
                  </TooltipContent>
                </Tooltip>
              )}

              {workflowProgress &&
                workflowProgress.steps.length > 0 &&
                showWorkflowRail && (
                  <div className="overflow-hidden max-h-80 opacity-100 pointer-events-auto transition-all duration-200">
                    <div className="w-8 border-t my-1" />
                    <div className="flex flex-col items-center gap-1">
                      {workflowProgress.steps.map((step, index) => {
                        const isCurrent = index === workflowProgress.currentIndex;
                        const isCompleted =
                          step.status === "completed" || step.status === "skipped";
                        return (
                          <Tooltip key={step.id}>
                            <TooltipTrigger asChild>
                              <button
                                type="button"
                                className={cn(
                                  "h-7 w-7 rounded-full border text-[11px] font-medium transition-colors",
                                  isCurrent &&
                                    "border-primary bg-primary/10 text-primary",
                                  !isCurrent &&
                                    isCompleted &&
                                    "border-primary/40 bg-primary/5 text-primary",
                                  !isCurrent &&
                                    !isCompleted &&
                                    "border-border bg-muted/40 text-muted-foreground",
                                )}
                              >
                                {isCompleted ? "✓" : index + 1}
                              </button>
                            </TooltipTrigger>
                            <TooltipContent side="left">
                              <p>{step.title}</p>
                              <p className="text-[11px] text-muted-foreground">
                                {isCurrent
                                  ? "当前步骤"
                                  : getWorkflowStepStatusLabel(step.status)}
                              </p>
                            </TooltipContent>
                          </Tooltip>
                        );
                      })}
                    </div>
                  </div>
                )}
            </>
          ) : (
            <>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-9 w-9"
                    onClick={onBackToCreateView}
                    title="返回创作视图"
                  >
                    <Bot className="h-4 w-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="left">
                  <p>返回创作视图</p>
                </TooltipContent>
              </Tooltip>

              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    className={cn(
                      "h-9 w-9",
                      activeRightDrawer === "tools" &&
                        "bg-accent text-accent-foreground",
                    )}
                    onClick={onToggleToolsDrawer}
                    title={
                      activeRightDrawer === "tools" ? "收起视图动作" : "展开视图动作"
                    }
                  >
                    <Wrench className="h-4 w-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="left">
                  <p>
                    {activeRightDrawer === "tools" ? "收起视图动作" : "展开视图动作"}
                  </p>
                </TooltipContent>
              </Tooltip>
            </>
          )}
        </TooltipProvider>
      </aside>
    </>
  );
}

export default WorkbenchRightRail;
