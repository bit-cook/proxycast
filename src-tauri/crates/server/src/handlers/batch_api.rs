//! 批量任务 API 端点
//!
//! 提供批量任务的创建、查询和管理接口

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use proxycast_scheduler::{
    BatchOptions, BatchTask, BatchTaskDao, BatchTaskStatistics, TaskDefinition, TaskTemplate,
    TemplateDao,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

/// 创建批量任务请求
#[derive(Debug, Deserialize)]
pub struct CreateBatchTaskRequest {
    /// 批量任务名称
    pub name: String,

    /// 任务模板 ID
    pub template_id: Uuid,

    /// 任务列表
    pub tasks: Vec<TaskDefinition>,

    /// 批量任务选项
    #[serde(default)]
    pub options: BatchOptions,
}

/// 创建批量任务响应
#[derive(Debug, Serialize)]
pub struct CreateBatchTaskResponse {
    /// 批量任务 ID
    pub id: Uuid,

    /// 批量任务名称
    pub name: String,

    /// 任务数量
    pub task_count: usize,

    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 批量任务详情响应
#[derive(Debug, Serialize)]
pub struct BatchTaskDetailResponse {
    /// 批量任务信息
    #[serde(flatten)]
    pub batch_task: BatchTask,

    /// 统计信息
    pub statistics: BatchTaskStatistics,
}

/// POST /api/batch/tasks - 创建批量任务
pub async fn create_batch_task(
    State(state): State<AppState>,
    Json(request): Json<CreateBatchTaskRequest>,
) -> Response {
    // 检查数据库是否可用
    let db = match &state.db {
        Some(db) => db,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "message": "数据库未初始化",
                        "type": "database_error"
                    }
                })),
            )
                .into_response();
        }
    };

    // 验证模板是否存在
    let template = match TemplateDao::get_by_id(db, &request.template_id) {
        Ok(Some(t)) => t,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": {
                        "message": format!("模板不存在: {}", request.template_id),
                        "type": "not_found"
                    }
                })),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "message": format!("查询模板失败: {}", e),
                        "type": "database_error"
                    }
                })),
            )
                .into_response();
        }
    };

    // 创建批量任务
    let batch_task = BatchTask::new(
        request.name.clone(),
        request.template_id,
        request.tasks,
        request.options,
    );

    let batch_id = batch_task.id;
    let task_count = batch_task.tasks.len();
    let created_at = batch_task.created_at;

    // 保存到数据库
    if let Err(e) = BatchTaskDao::save(db, &batch_task) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": {
                    "message": format!("保存批量任务失败: {}", e),
                    "type": "database_error"
                }
            })),
        )
            .into_response();
    }

    state.logs.write().await.add(
        "info",
        &format!(
            "[BATCH] 创建批量任务: id={}, name={}, task_count={}",
            batch_id, request.name, task_count
        ),
    );

    // TODO: 启动异步执行任务
    // 这里需要集成 BatchTaskExecutor

    // 返回响应
    (
        StatusCode::CREATED,
        Json(CreateBatchTaskResponse {
            id: batch_id,
            name: request.name,
            task_count,
            created_at,
        }),
    )
        .into_response()
}

/// GET /api/batch/tasks/:id - 获取批量任务详情
pub async fn get_batch_task(State(state): State<AppState>, Path(id): Path<Uuid>) -> Response {
    let db = match &state.db {
        Some(db) => db,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "message": "数据库未初始化",
                        "type": "database_error"
                    }
                })),
            )
                .into_response();
        }
    };

    match BatchTaskDao::get_by_id(db, &id) {
        Ok(Some(batch_task)) => {
            let statistics = batch_task.get_statistics();
            (
                StatusCode::OK,
                Json(BatchTaskDetailResponse {
                    batch_task,
                    statistics,
                }),
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": {
                    "message": format!("批量任务不存在: {}", id),
                    "type": "not_found"
                }
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": {
                    "message": format!("查询批量任务失败: {}", e),
                    "type": "database_error"
                }
            })),
        )
            .into_response(),
    }
}

/// GET /api/batch/tasks - 获取批量任务列表
pub async fn list_batch_tasks(State(state): State<AppState>) -> Response {
    let db = match &state.db {
        Some(db) => db,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "message": "数据库未初始化",
                        "type": "database_error"
                    }
                })),
            )
                .into_response();
        }
    };

    match BatchTaskDao::list_all(db, 100) {
        Ok(tasks) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "tasks": tasks
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": {
                    "message": format!("查询批量任务列表失败: {}", e),
                    "type": "database_error"
                }
            })),
        )
            .into_response(),
    }
}

/// DELETE /api/batch/tasks/:id - 取消批量任务
pub async fn cancel_batch_task(State(state): State<AppState>, Path(id): Path<Uuid>) -> Response {
    // TODO: 取消批量任务
    state
        .logs
        .write()
        .await
        .add("info", &format!("[BATCH] 取消批量任务: id={}", id));

    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error": {
                "message": format!("批量任务不存在: {}", id),
                "type": "not_found"
            }
        })),
    )
        .into_response()
}

/// POST /api/batch/templates - 创建任务模板
pub async fn create_template(
    State(state): State<AppState>,
    Json(template): Json<TaskTemplate>,
) -> Response {
    let db = match &state.db {
        Some(db) => db,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "message": "数据库未初始化",
                        "type": "database_error"
                    }
                })),
            )
                .into_response();
        }
    };

    if let Err(e) = TemplateDao::save(db, &template) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": {
                    "message": format!("保存模板失败: {}", e),
                    "type": "database_error"
                }
            })),
        )
            .into_response();
    }

    state.logs.write().await.add(
        "info",
        &format!(
            "[BATCH] 创建任务模板: id={}, name={}",
            template.id, template.name
        ),
    );

    (StatusCode::CREATED, Json(template)).into_response()
}

/// GET /api/batch/templates - 获取模板列表
pub async fn list_templates(State(state): State<AppState>) -> Response {
    let db = match &state.db {
        Some(db) => db,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "message": "数据库未初始化",
                        "type": "database_error"
                    }
                })),
            )
                .into_response();
        }
    };

    match TemplateDao::list_all(db) {
        Ok(templates) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "templates": templates
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": {
                    "message": format!("查询模板列表失败: {}", e),
                    "type": "database_error"
                }
            })),
        )
            .into_response(),
    }
}

/// GET /api/batch/templates/:id - 获取模板详情
pub async fn get_template(State(state): State<AppState>, Path(id): Path<Uuid>) -> Response {
    let db = match &state.db {
        Some(db) => db,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "message": "数据库未初始化",
                        "type": "database_error"
                    }
                })),
            )
                .into_response();
        }
    };

    match TemplateDao::get_by_id(db, &id) {
        Ok(Some(template)) => (StatusCode::OK, Json(template)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": {
                    "message": format!("模板不存在: {}", id),
                    "type": "not_found"
                }
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": {
                    "message": format!("查询模板失败: {}", e),
                    "type": "database_error"
                }
            })),
        )
            .into_response(),
    }
}

/// DELETE /api/batch/templates/:id - 删除模板
pub async fn delete_template(State(state): State<AppState>, Path(id): Path<Uuid>) -> Response {
    let db = match &state.db {
        Some(db) => db,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "message": "数据库未初始化",
                        "type": "database_error"
                    }
                })),
            )
                .into_response();
        }
    };

    match TemplateDao::delete(db, &id) {
        Ok(true) => {
            state
                .logs
                .write()
                .await
                .add("info", &format!("[BATCH] 删除模板: id={}", id));
            (StatusCode::NO_CONTENT, ()).into_response()
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": {
                    "message": format!("模板不存在: {}", id),
                    "type": "not_found"
                }
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": {
                    "message": format!("删除模板失败: {}", e),
                    "type": "database_error"
                }
            })),
        )
            .into_response(),
    }
}
