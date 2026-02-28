import NovelFlowWorkbench from "@/components/projects/tabs/novel-flow/NovelFlowWorkbench";
import { NovelPublishTab } from "@/components/projects/tabs/NovelPublishTab";
import type { ThemeWorkspaceRendererProps } from "@/features/themes/types";

export function NovelWorkflowPanel({
  projectId,
  projectName,
}: ThemeWorkspaceRendererProps) {
  if (!projectId) {
    return null;
  }
  return <NovelFlowWorkbench projectId={projectId} projectName={projectName} />;
}

export function NovelPublishPanel({
  projectId,
}: ThemeWorkspaceRendererProps) {
  if (!projectId) {
    return null;
  }
  return <NovelPublishTab projectId={projectId} />;
}
