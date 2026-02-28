import {
  checkNovelConsistency,
  continueNovelChapter,
  generateNovelChapter,
  type NovelChapterRecord,
  type NovelConsistencyCheck,
} from "@/lib/api/novel";

export interface GenerateNextChapterFlowParams {
  projectId: string;
  hasExistingChapters: boolean;
  provider?: string;
  model?: string;
}

export interface GenerateNextChapterFlowResult {
  chapter: NovelChapterRecord;
  consistency?: NovelConsistencyCheck;
  consistencyError?: string;
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "string") {
    return error;
  }
  return "未知错误";
}

/**
 * 章节半自动编排：生成下一章并尝试执行一致性检查。
 * 一致性检查失败不会回滚章节，交由上层做软告警展示。
 */
export async function generateNextChapterWithConsistency(
  params: GenerateNextChapterFlowParams,
): Promise<GenerateNextChapterFlowResult> {
  const generationResult = params.hasExistingChapters
    ? await continueNovelChapter({
        project_id: params.projectId,
        provider: params.provider,
        model: params.model,
      })
    : await generateNovelChapter({
        project_id: params.projectId,
        provider: params.provider,
        model: params.model,
      });

  const chapter = generationResult.chapter;
  if (!chapter) {
    throw new Error("章节生成成功但未返回章节数据");
  }

  try {
    const consistency = await checkNovelConsistency({
      project_id: params.projectId,
      chapter_id: chapter.id,
    });
    return {
      chapter,
      consistency,
    };
  } catch (error) {
    return {
      chapter,
      consistencyError: getErrorMessage(error),
    };
  }
}
