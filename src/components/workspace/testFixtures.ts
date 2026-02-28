import type { ContentListItem, Project } from "@/lib/api/project";

export const WORKSPACE_FIXTURE_TIMESTAMP = 1_700_000_000_000;

export const DEFAULT_WORKSPACE_PAGE_PROPS = {
  viewMode: "workspace",
  projectId: "project-1",
  contentId: "content-1",
} as const;

type ProjectFixtureRequired = Pick<
  Project,
  "id" | "name" | "workspaceType" | "rootPath"
>;

export function createWorkspaceProjectFixture(
  data: ProjectFixtureRequired & Partial<Project>,
): Project {
  return {
    isDefault: false,
    createdAt: WORKSPACE_FIXTURE_TIMESTAMP,
    updatedAt: WORKSPACE_FIXTURE_TIMESTAMP,
    isFavorite: false,
    isArchived: false,
    tags: [],
    ...data,
  };
}

type ContentFixtureRequired = Pick<ContentListItem, "id" | "project_id" | "title">;

export function createWorkspaceContentFixture(
  data: ContentFixtureRequired & Partial<ContentListItem>,
): ContentListItem {
  return {
    content_type: "post",
    status: "draft",
    order: 0,
    word_count: 0,
    created_at: WORKSPACE_FIXTURE_TIMESTAMP,
    updated_at: WORKSPACE_FIXTURE_TIMESTAMP,
    ...data,
  };
}
