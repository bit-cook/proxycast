//! 视频生成命令
//!
//! 兼容层：对外保持 tauri command 名称不变，内部转发到主题模块实现。

use tauri::State;

use crate::commands::api_key_provider_cmd::ApiKeyProviderServiceState;
use crate::database::DbConnection;
pub use crate::theme::video::command::{
    CancelVideoTaskRequest, GetVideoTaskRequest, ListVideoTasksRequest,
};
use proxycast_core::database::dao::video_generation_task_dao::VideoGenerationTask;
use proxycast_services::video_generation_service::CreateVideoGenerationRequest;

#[tauri::command]
pub async fn create_video_generation_task(
    db: State<'_, DbConnection>,
    api_key_provider_service: State<'_, ApiKeyProviderServiceState>,
    request: CreateVideoGenerationRequest,
) -> Result<VideoGenerationTask, String> {
    crate::theme::video::command::create_video_generation_task(
        db,
        api_key_provider_service,
        request,
    )
    .await
}

#[tauri::command]
pub async fn get_video_generation_task(
    db: State<'_, DbConnection>,
    api_key_provider_service: State<'_, ApiKeyProviderServiceState>,
    request: GetVideoTaskRequest,
) -> Result<Option<VideoGenerationTask>, String> {
    crate::theme::video::command::get_video_generation_task(db, api_key_provider_service, request)
        .await
}

#[tauri::command]
pub fn list_video_generation_tasks(
    db: State<'_, DbConnection>,
    request: ListVideoTasksRequest,
) -> Result<Vec<VideoGenerationTask>, String> {
    crate::theme::video::command::list_video_generation_tasks(db, request)
}

#[tauri::command]
pub async fn cancel_video_generation_task(
    db: State<'_, DbConnection>,
    api_key_provider_service: State<'_, ApiKeyProviderServiceState>,
    request: CancelVideoTaskRequest,
) -> Result<Option<VideoGenerationTask>, String> {
    crate::theme::video::command::cancel_video_generation_task(
        db,
        api_key_provider_service,
        request,
    )
    .await
}
