//! 视频主题命令适配层

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::api_key_provider_cmd::ApiKeyProviderServiceState;
use crate::database::DbConnection;
use proxycast_core::database::dao::video_generation_task_dao::VideoGenerationTask;
use proxycast_services::video_generation_service::{
    CreateVideoGenerationRequest, VideoGenerationService,
};

static VIDEO_GENERATION_SERVICE: Lazy<VideoGenerationService> =
    Lazy::new(VideoGenerationService::new);

/// 获取视频任务请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetVideoTaskRequest {
    pub task_id: String,
    pub refresh_status: Option<bool>,
}

/// 列表视频任务请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListVideoTasksRequest {
    pub project_id: String,
    pub limit: Option<i64>,
}

/// 取消视频任务请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelVideoTaskRequest {
    pub task_id: String,
}

pub async fn create_video_generation_task(
    db: State<'_, DbConnection>,
    api_key_provider_service: State<'_, ApiKeyProviderServiceState>,
    request: CreateVideoGenerationRequest,
) -> Result<VideoGenerationTask, String> {
    VIDEO_GENERATION_SERVICE
        .create_task(&db, &api_key_provider_service.0, request)
        .await
}

pub async fn get_video_generation_task(
    db: State<'_, DbConnection>,
    api_key_provider_service: State<'_, ApiKeyProviderServiceState>,
    request: GetVideoTaskRequest,
) -> Result<Option<VideoGenerationTask>, String> {
    VIDEO_GENERATION_SERVICE
        .get_task(
            &db,
            &api_key_provider_service.0,
            &request.task_id,
            request.refresh_status.unwrap_or(true),
        )
        .await
}

pub fn list_video_generation_tasks(
    db: State<'_, DbConnection>,
    request: ListVideoTasksRequest,
) -> Result<Vec<VideoGenerationTask>, String> {
    VIDEO_GENERATION_SERVICE.list_tasks(
        &db,
        &request.project_id,
        request.limit.unwrap_or(50).clamp(1, 200),
    )
}

pub async fn cancel_video_generation_task(
    db: State<'_, DbConnection>,
    api_key_provider_service: State<'_, ApiKeyProviderServiceState>,
    request: CancelVideoTaskRequest,
) -> Result<Option<VideoGenerationTask>, String> {
    VIDEO_GENERATION_SERVICE
        .cancel_task(&db, &api_key_provider_service.0, &request.task_id)
        .await
}
