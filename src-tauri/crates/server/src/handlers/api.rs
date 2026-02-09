//! API 端点处理器
//!
//! 处理 OpenAI 和 Anthropic 格式的 API 请求
//!
//! # 流式传输支持

#![allow(dead_code)]
//!
//! 本模块支持真正的端到端流式传输：
//! - 对于流式请求，使用 StreamManager 处理响应
//! - 集成 Flow Monitor 实时捕获流式内容
//!
//! # 需求覆盖
//!
//! - 需求 5.1: 在收到 chunk 后立即转发给客户端
//! - 需求 5.3: 流中发生错误时发送错误事件并优雅关闭流

use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::client_detector::ClientType;
use crate::{record_request_telemetry, record_token_usage, AppState};
use proxycast_core::models::anthropic::AnthropicMessagesRequest;
use proxycast_core::models::openai::ChatCompletionRequest;
use proxycast_core::ProviderType;
use proxycast_processor::RequestContext;
use proxycast_providers::converter::anthropic_to_openai::convert_anthropic_to_openai;
use proxycast_providers::streaming::StreamFormat as StreamingFormat;
use proxycast_server_utils::{
    build_anthropic_response, build_anthropic_stream_response, message_content_len,
    parse_cw_response, safe_truncate,
};

use super::{call_provider_anthropic, call_provider_openai};

async fn select_credential_for_request(
    state: &AppState,
    selected_provider: &str,
    model: &str,
    client_type: &ClientType,
    explicit_provider_id: Option<&str>,
    log_prefix: &str,
    include_error_code: bool,
) -> Result<Option<proxycast_core::models::provider_pool_model::ProviderCredential>, Response> {
    let db = match &state.db {
        Some(db) => db,
        None => {
            eprintln!("[{log_prefix}] 数据库未初始化!");
            return Ok(None);
        }
    };

    if let Some(explicit_provider_id) = explicit_provider_id {
        eprintln!("[{log_prefix}] 使用 X-Provider-Id 指定的 provider: {explicit_provider_id}");
        let cred = state
            .pool_service
            .select_credential_with_client_check(
                db,
                explicit_provider_id,
                Some(model),
                Some(client_type),
            )
            .ok()
            .flatten();

        if cred.is_none() {
            eprintln!(
                "[{log_prefix}] X-Provider-Id '{explicit_provider_id}' 没有可用凭证，不进行降级"
            );
            state.logs.write().await.add(
                "error",
                &format!(
                    "[ROUTE] No available credentials for explicitly specified provider '{explicit_provider_id}', refusing to fallback"
                ),
            );

            let mut error_body = json!({
                "error": {
                    "type": "provider_unavailable",
                    "message": format!("No available credentials for provider '{}'", explicit_provider_id)
                }
            });
            if include_error_code {
                error_body["error"]["code"] = json!("no_credentials");
            }

            return Err((StatusCode::SERVICE_UNAVAILABLE, Json(error_body)).into_response());
        }

        return Ok(cred);
    }

    let provider_id_hint = selected_provider.to_lowercase();
    match state
        .pool_service
        .select_credential_with_fallback(
            db,
            &state.api_key_service,
            selected_provider,
            Some(model),
            Some(provider_id_hint.as_str()),
            Some(client_type),
        )
        .await
    {
        Ok(cred) => {
            if cred.is_some() {
                eprintln!("[{log_prefix}] 找到凭证: provider={selected_provider}");
            } else {
                eprintln!("[{log_prefix}] 未找到凭证: provider={selected_provider}");
            }
            Ok(cred)
        }
        Err(e) => {
            eprintln!("[{log_prefix}] 选择凭证失败: {e}");
            Ok(None)
        }
    }
}

// ============================================================================
// Provider 选择辅助函数
// ============================================================================

/// 根据客户端类型和端点配置选择 Provider
async fn select_provider_for_client(headers: &HeaderMap, state: &AppState) -> (String, ClientType) {
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let client_type = ClientType::from_user_agent(user_agent);

    let endpoint_providers = state.endpoint_providers.read().await;
    let endpoint_provider = endpoint_providers.get_provider(client_type.config_key());

    let default_provider = state.default_provider.read().await.clone();

    let selected_provider = match endpoint_provider {
        Some(provider) => provider.clone(),
        None => default_provider,
    };

    (selected_provider, client_type)
}

// ============================================================================
// API Key 验证
// ============================================================================

/// OpenAI 格式的 API key 验证
pub async fn verify_api_key(
    headers: &HeaderMap,
    expected_key: &str,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let auth = headers
        .get("authorization")
        .or_else(|| headers.get("x-api-key"))
        .and_then(|v| v.to_str().ok());

    let key = match auth {
        Some(s) if s.starts_with("Bearer ") => &s[7..],
        Some(s) => s,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": {"message": "No API key provided"}})),
            ))
        }
    };

    if key != expected_key {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": {"message": "Invalid API key"}})),
        ));
    }

    Ok(())
}

/// Anthropic 格式的 API key 验证
pub async fn verify_api_key_anthropic(
    headers: &HeaderMap,
    expected_key: &str,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let auth = headers
        .get("x-api-key")
        .or_else(|| headers.get("authorization"))
        .and_then(|v| v.to_str().ok());

    let key = match auth {
        Some(s) if s.starts_with("Bearer ") => &s[7..],
        Some(s) => s,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "type": "error",
                    "error": {
                        "type": "authentication_error",
                        "message": "No API key provided. Please set the x-api-key header."
                    }
                })),
            ))
        }
    };

    if key != expected_key {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "type": "error",
                "error": {
                    "type": "authentication_error",
                    "message": "Invalid API key"
                }
            })),
        ));
    }

    Ok(())
}

pub async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut request): Json<ChatCompletionRequest>,
) -> Response {
    // ========== 详细日志：请求入口 ==========
    eprintln!("\n========== [CHAT_COMPLETIONS] 收到请求 ==========");
    eprintln!("[CHAT_COMPLETIONS] URL: /v1/chat/completions");
    eprintln!("[CHAT_COMPLETIONS] 模型: {}", request.model);
    eprintln!("[CHAT_COMPLETIONS] 流式: {}", request.stream);
    eprintln!("[CHAT_COMPLETIONS] 消息数量: {}", request.messages.len());

    if let Err(e) = verify_api_key(&headers, &state.api_key).await {
        eprintln!("[CHAT_COMPLETIONS] 认证失败!");
        state
            .logs
            .write()
            .await
            .add("warn", "Unauthorized request to /v1/chat/completions");
        return e.into_response();
    }
    eprintln!("[CHAT_COMPLETIONS] 认证成功");

    // 创建请求上下文
    let mut ctx = RequestContext::new(request.model.clone()).with_stream(request.stream);
    eprintln!("[CHAT_COMPLETIONS] 请求ID: {}", ctx.request_id);

    state.logs.write().await.add(
        "info",
        &format!(
            "POST /v1/chat/completions request_id={} model={} stream={}",
            ctx.request_id, request.model, request.stream
        ),
    );

    // 使用 RequestProcessor 解析模型别名
    eprintln!("[CHAT_COMPLETIONS] 开始模型别名解析...");
    let resolved_model = state.processor.resolve_model(&request.model).await;
    ctx.set_resolved_model(resolved_model.clone());
    eprintln!(
        "[CHAT_COMPLETIONS] 模型别名解析结果: {} -> {}",
        request.model, resolved_model
    );

    // 更新请求中的模型名为解析后的模型
    if resolved_model != request.model {
        request.model = resolved_model.clone();
        state.logs.write().await.add(
            "info",
            &format!(
                "[MAPPER] request_id={} alias={} -> model={}",
                ctx.request_id, ctx.original_model, resolved_model
            ),
        );
    }

    // 应用参数注入
    let injection_enabled = *state.injection_enabled.read().await;
    if injection_enabled {
        let injector = state.processor.injector.read().await;
        let mut payload = serde_json::to_value(&request).unwrap_or_default();
        let result = injector.inject(&request.model, &mut payload);
        if result.has_injections() {
            state.logs.write().await.add(
                "info",
                &format!(
                    "[INJECT] request_id={} applied_rules={:?} injected_params={:?}",
                    ctx.request_id, result.applied_rules, result.injected_params
                ),
            );
            // 更新请求
            if let Ok(updated) = serde_json::from_value(payload) {
                request = updated;
            }
        }
    }

    // 根据客户端类型选择 Provider
    // **Validates: Requirements 3.1, 3.3, 3.4**
    let (selected_provider, client_type) = select_provider_for_client(&headers, &state).await;
    eprintln!("[CHAT_COMPLETIONS] 客户端类型: {client_type}, 选择的Provider: {selected_provider}");

    // 记录客户端检测和 Provider 选择结果
    state.logs.write().await.add(
        "info",
        &format!(
            "[CLIENT] request_id={} client_type={} selected_provider={}",
            ctx.request_id, client_type, selected_provider
        ),
    );

    // 记录路由结果（使用 selected_provider）
    state.logs.write().await.add(
        "info",
        &format!(
            "[ROUTE] request_id={} model={} provider={}",
            ctx.request_id, ctx.resolved_model, selected_provider
        ),
    );

    // 从请求头提取 X-Provider-Id（用于精确路由）
    let provider_id_header = headers
        .get("x-provider-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_lowercase());

    // 尝试选择凭证：
    // 1) X-Provider-Id 指定时仅走精确匹配（不降级）
    // 2) 否则走统一的“池优先 + API Key Provider 智能降级”路径
    eprintln!("[CHAT_COMPLETIONS] 开始选择凭证...");
    let credential = match select_credential_for_request(
        &state,
        &selected_provider,
        &request.model,
        &client_type,
        provider_id_header.as_deref(),
        "CHAT_COMPLETIONS",
        true,
    )
    .await
    {
        Ok(cred) => cred,
        Err(resp) => return resp,
    };

    // 如果找到凭证池中的凭证，使用它
    if let Some(cred) = credential {
        eprintln!(
            "[CHAT_COMPLETIONS] 使用凭证: type={}, name={:?}, uuid={}",
            cred.provider_type,
            cred.name,
            &cred.uuid[..8.min(cred.uuid.len())]
        );
        state.logs.write().await.add(
            "info",
            &format!(
                "[ROUTE] Using pool credential: type={} name={:?} uuid={}",
                cred.provider_type,
                cred.name,
                &cred.uuid[..8]
            ),
        );

        // 启动 Flow 捕获

        // 尝试将 selected_provider 解析为 ProviderType
        // 构建 Flow Metadata，同时保存 provider_type 和实际的 provider_id
        let _provider_type = selected_provider
            .parse::<ProviderType>()
            .unwrap_or(ProviderType::OpenAI);

        // 从凭证名称中提取 Provider 显示名称
        // 凭证名称格式：Some("[降级] DeepSeek") 或 Some("DeepSeek")
        let _provider_display_name = cred.name.as_ref().map(|name| {
            // 去掉 "[降级] " 前缀
            if name.starts_with("[降级] ") {
                &name[9..] // "[降级] " 是 9 个字节
            } else {
                name.as_str()
            }
        });

        // 检查是否需要拦截请求
        // **Validates: Requirements 2.1, 2.3, 2.5**

        eprintln!("[CHAT_COMPLETIONS] 调用 Provider: {}", cred.provider_type);
        let response = call_provider_openai(&state, &cred, &request, None).await;
        eprintln!(
            "[CHAT_COMPLETIONS] Provider 响应状态: {}",
            response.status()
        );

        // 记录请求统计
        let is_success = response.status().is_success();
        let _status_code = response.status().as_u16();
        let status = if is_success {
            proxycast_infra::telemetry::RequestStatus::Success
        } else {
            proxycast_infra::telemetry::RequestStatus::Failed
        };
        record_request_telemetry(&state, &ctx, status, None);

        // 如果成功且需要 Flow 捕获，提取响应体内容和响应头
        // 注意：非流式响应需要读取 body，所以必须在这里处理
    }

    // 回退到旧的单凭证模式（仅当选择的 Provider 是 Kiro 时）
    // 如果选择的 Provider 不是 Kiro，且凭证池中没有找到凭证，返回错误
    // **Validates: Requirements 3.2**
    if selected_provider.to_lowercase() != "kiro" {
        state.logs.write().await.add(
            "error",
            &format!(
                "[ROUTE] No pool credential found for '{selected_provider}' (client_type={client_type}), and legacy mode only supports Kiro"
            ),
        );
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": {
                    "message": format!("没有找到可用的 '{}' 凭证。请在凭证池中添加对应的凭证。", selected_provider),
                    "type": "no_credential_error",
                    "code": "no_credential"
                }
            })),
        )
            .into_response();
    }

    state.logs.write().await.add(
        "debug",
        &format!("[ROUTE] No pool credential found for '{selected_provider}', using legacy mode"),
    );

    // 启动 Flow 捕获（legacy mode）

    // 使用实际的 provider ID 构建 Flow Metadata
    let _provider_type = selected_provider
        .parse::<ProviderType>()
        .unwrap_or(ProviderType::OpenAI);

    // 检查是否需要拦截请求（legacy mode）
    // **Validates: Requirements 2.1, 2.3, 2.5**

    // 检查是否需要刷新 token（无 token 或即将过期）
    {
        let _guard = state.kiro_refresh_lock.lock().await;
        let mut kiro = state.kiro.write().await;
        let needs_refresh =
            kiro.credentials.access_token.is_none() || kiro.is_token_expiring_soon();
        if needs_refresh {
            if let Err(e) = kiro.refresh_token().await {
                state
                    .logs
                    .write()
                    .await
                    .add("error", &format!("Token refresh failed: {e}"));
                // 标记 Flow 失败
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({"error": {"message": format!("Token refresh failed: {e}")}})),
                ).into_response();
            }
        }
    }

    let kiro = state.kiro.read().await;

    match kiro.call_api(&request).await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                match resp.text().await {
                    Ok(body) => {
                        let parsed = parse_cw_response(&body);
                        let has_tool_calls = !parsed.tool_calls.is_empty();

                        state.logs.write().await.add(
                            "info",
                            &format!(
                                "Request completed: content_len={}, tool_calls={}",
                                parsed.content.len(),
                                parsed.tool_calls.len()
                            ),
                        );

                        // 构建消息
                        let message = if has_tool_calls {
                            serde_json::json!({
                                "role": "assistant",
                                "content": if parsed.content.is_empty() { serde_json::Value::Null } else { serde_json::json!(parsed.content) },
                                "tool_calls": parsed.tool_calls.iter().map(|tc| {
                                    serde_json::json!({
                                        "id": tc.id,
                                        "type": "function",
                                        "function": {
                                            "name": tc.function.name,
                                            "arguments": tc.function.arguments
                                        }
                                    })
                                }).collect::<Vec<_>>()
                            })
                        } else {
                            serde_json::json!({
                                "role": "assistant",
                                "content": parsed.content
                            })
                        };

                        // 估算 Token 数量（基于字符数，约 4 字符 = 1 token）
                        let estimated_output_tokens = (parsed.content.len() / 4) as u32;
                        // 估算输入 Token（基于请求消息）
                        let estimated_input_tokens = request
                            .messages
                            .iter()
                            .map(|m| {
                                let content_len = match &m.content {
                                    Some(c) => message_content_len(c),
                                    None => 0,
                                };
                                content_len / 4
                            })
                            .sum::<usize>()
                            as u32;

                        let response = serde_json::json!({
                            "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                            "object": "chat.completion",
                            "created": std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                            "model": request.model,
                            "choices": [{
                                "index": 0,
                                "message": message,
                                "finish_reason": if has_tool_calls { "tool_calls" } else { "stop" }
                            }],
                            "usage": {
                                "prompt_tokens": estimated_input_tokens,
                                "completion_tokens": estimated_output_tokens,
                                "total_tokens": estimated_input_tokens + estimated_output_tokens
                            }
                        });
                        // 记录成功请求统计
                        record_request_telemetry(
                            &state,
                            &ctx,
                            proxycast_infra::telemetry::RequestStatus::Success,
                            None,
                        );
                        // 记录 Token 使用量
                        record_token_usage(
                            &state,
                            &ctx,
                            Some(estimated_input_tokens),
                            Some(estimated_output_tokens),
                        );
                        // 完成 Flow 捕获并检查响应拦截
                        // **Validates: Requirements 2.1, 2.5**
                        Json(response).into_response()
                    }
                    Err(e) => {
                        // 记录失败请求统计
                        record_request_telemetry(
                            &state,
                            &ctx,
                            proxycast_infra::telemetry::RequestStatus::Failed,
                            Some(e.to_string()),
                        );
                        // 标记 Flow 失败
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({"error": {"message": e.to_string()}})),
                        )
                            .into_response()
                    }
                }
            } else if status.as_u16() == 403 || status.as_u16() == 402 {
                // Token 过期或账户问题，尝试重新加载凭证并刷新
                drop(kiro);
                let _guard = state.kiro_refresh_lock.lock().await;
                let mut kiro = state.kiro.write().await;
                state.logs.write().await.add(
                    "warn",
                    &format!(
                        "[AUTH] Got {}, reloading credentials and attempting token refresh...",
                        status.as_u16()
                    ),
                );

                // 先重新加载凭证文件（可能用户换了账户）
                if let Err(e) = kiro.load_credentials().await {
                    state.logs.write().await.add(
                        "error",
                        &format!("[AUTH] Failed to reload credentials: {e}"),
                    );
                }

                match kiro.refresh_token().await {
                    Ok(_) => {
                        state
                            .logs
                            .write()
                            .await
                            .add("info", "[AUTH] Token refreshed successfully after reload");
                        // 重试请求
                        drop(kiro);
                        let kiro = state.kiro.read().await;
                        match kiro.call_api(&request).await {
                            Ok(retry_resp) => {
                                if retry_resp.status().is_success() {
                                    match retry_resp.text().await {
                                        Ok(body) => {
                                            let parsed = parse_cw_response(&body);
                                            let has_tool_calls = !parsed.tool_calls.is_empty();

                                            let message = if has_tool_calls {
                                                serde_json::json!({
                                                    "role": "assistant",
                                                    "content": if parsed.content.is_empty() { serde_json::Value::Null } else { serde_json::json!(parsed.content) },
                                                    "tool_calls": parsed.tool_calls.iter().map(|tc| {
                                                        serde_json::json!({
                                                            "id": tc.id,
                                                            "type": "function",
                                                            "function": {
                                                                "name": tc.function.name,
                                                                "arguments": tc.function.arguments
                                                            }
                                                        })
                                                    }).collect::<Vec<_>>()
                                                })
                                            } else {
                                                serde_json::json!({
                                                    "role": "assistant",
                                                    "content": parsed.content
                                                })
                                            };

                                            let response = serde_json::json!({
                                                "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                                                "object": "chat.completion",
                                                "created": std::time::SystemTime::now()
                                                    .duration_since(std::time::UNIX_EPOCH)
                                                    .unwrap_or_default()
                                                    .as_secs(),
                                                "model": request.model,
                                                "choices": [{
                                                    "index": 0,
                                                    "message": message,
                                                    "finish_reason": if has_tool_calls { "tool_calls" } else { "stop" }
                                                }],
                                                "usage": {
                                                    "prompt_tokens": 0,
                                                    "completion_tokens": 0,
                                                    "total_tokens": 0
                                                }
                                            });
                                            // 完成 Flow 捕获并检查响应拦截（重试成功）
                                            // **Validates: Requirements 2.1, 2.5**
                                            return Json(response).into_response();
                                        }
                                        Err(e) => {
                                            // 标记 Flow 失败
                                            return (
                                            StatusCode::INTERNAL_SERVER_ERROR,
                                            Json(serde_json::json!({"error": {"message": e.to_string()}})),
                                        ).into_response();
                                        }
                                    }
                                }
                                let body = retry_resp.text().await.unwrap_or_default();
                                // 标记 Flow 失败（重试失败）
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(serde_json::json!({"error": {"message": format!("Retry failed: {}", body)}})),
                                ).into_response()
                            }
                            Err(e) => {
                                // 标记 Flow 失败
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(serde_json::json!({"error": {"message": e.to_string()}})),
                                )
                                    .into_response()
                            }
                        }
                    }
                    Err(e) => {
                        state
                            .logs
                            .write()
                            .await
                            .add("error", &format!("[AUTH] Token refresh failed: {e}"));
                        // 标记 Flow 失败
                        (
                            StatusCode::UNAUTHORIZED,
                            Json(serde_json::json!({"error": {"message": format!("Token refresh failed: {e}")}})),
                        )
                            .into_response()
                    }
                }
            } else {
                let body = resp.text().await.unwrap_or_default();
                state.logs.write().await.add(
                    "error",
                    &format!("Upstream error {}: {}", status, safe_truncate(&body, 200)),
                );
                // 标记 Flow 失败
                (
                    StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                    Json(serde_json::json!({"error": {"message": format!("Upstream error: {}", body)}}))
                ).into_response()
            }
        }
        Err(e) => {
            state
                .logs
                .write()
                .await
                .add("error", &format!("API call failed: {e}"));
            // 标记 Flow 失败
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": {"message": e.to_string()}})),
            )
                .into_response()
        }
    }
}

pub async fn anthropic_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut request): Json<AnthropicMessagesRequest>,
) -> Response {
    // 使用 Anthropic 格式的认证验证（优先检查 x-api-key）
    if let Err(e) = verify_api_key_anthropic(&headers, &state.api_key).await {
        state
            .logs
            .write()
            .await
            .add("warn", "Unauthorized request to /v1/messages");
        return e.into_response();
    }

    // 创建请求上下文
    let mut ctx = RequestContext::new(request.model.clone()).with_stream(request.stream);

    // 详细记录请求信息
    let msg_count = request.messages.len();
    let has_tools = request.tools.as_ref().map(|t| t.len()).unwrap_or(0);
    let has_system = request.system.is_some();
    state.logs.write().await.add(
        "info",
        &format!(
            "[REQ] POST /v1/messages request_id={} model={} stream={} messages={} tools={} has_system={}",
            ctx.request_id, request.model, request.stream, msg_count, has_tools, has_system
        ),
    );

    // 使用 RequestProcessor 解析模型别名
    let resolved_model = state.processor.resolve_model(&request.model).await;
    ctx.set_resolved_model(resolved_model.clone());

    // 更新请求中的模型名为解析后的模型
    if resolved_model != request.model {
        request.model = resolved_model.clone();
        state.logs.write().await.add(
            "info",
            &format!(
                "[MAPPER] request_id={} alias={} -> model={}",
                ctx.request_id, ctx.original_model, resolved_model
            ),
        );
    }

    // 记录最后一条消息的角色和内容预览
    if let Some(last_msg) = request.messages.last() {
        let content_preview = match &last_msg.content {
            serde_json::Value::String(s) => s.chars().take(100).collect::<String>(),
            serde_json::Value::Array(arr) => {
                if let Some(first) = arr.first() {
                    if let Some(text) = first.get("text").and_then(|t| t.as_str()) {
                        text.chars().take(100).collect::<String>()
                    } else {
                        format!("[{} blocks]", arr.len())
                    }
                } else {
                    "[empty]".to_string()
                }
            }
            _ => "[unknown]".to_string(),
        };
        state.logs.write().await.add(
            "debug",
            &format!(
                "[REQ] request_id={} last_message: role={} content={}",
                ctx.request_id, last_msg.role, content_preview
            ),
        );
    }

    // 应用参数注入
    let injection_enabled = *state.injection_enabled.read().await;
    if injection_enabled {
        let injector = state.processor.injector.read().await;
        let mut payload = serde_json::to_value(&request).unwrap_or_default();
        let result = injector.inject(&request.model, &mut payload);
        if result.has_injections() {
            state.logs.write().await.add(
                "info",
                &format!(
                    "[INJECT] request_id={} applied_rules={:?} injected_params={:?}",
                    ctx.request_id, result.applied_rules, result.injected_params
                ),
            );
            // 更新请求
            if let Ok(updated) = serde_json::from_value(payload) {
                request = updated;
            }
        }
    }

    // 根据客户端类型选择 Provider
    // **Validates: Requirements 3.1, 3.3, 3.4**
    let (selected_provider, client_type) = select_provider_for_client(&headers, &state).await;

    // 记录客户端检测和 Provider 选择结果
    state.logs.write().await.add(
        "info",
        &format!(
            "[CLIENT] request_id={} client_type={} selected_provider={}",
            ctx.request_id, client_type, selected_provider
        ),
    );

    // 记录路由结果（使用 selected_provider）
    state.logs.write().await.add(
        "info",
        &format!(
            "[ROUTE] request_id={} model={} provider={}",
            ctx.request_id, ctx.resolved_model, selected_provider
        ),
    );

    // 从请求头提取 X-Provider-Id（用于精确路由）
    let provider_id_header = headers
        .get("x-provider-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_lowercase());

    // 尝试选择凭证：
    // 1) X-Provider-Id 指定时仅走精确匹配（不降级）
    // 2) 否则走统一的“池优先 + API Key Provider 智能降级”路径
    let credential = match select_credential_for_request(
        &state,
        &selected_provider,
        &request.model,
        &client_type,
        provider_id_header.as_deref(),
        "ANTHROPIC_MESSAGES",
        false,
    )
    .await
    {
        Ok(cred) => cred,
        Err(resp) => return resp,
    };

    // 如果找到凭证池中的凭证，使用它
    if let Some(cred) = credential {
        state.logs.write().await.add(
            "info",
            &format!(
                "[ROUTE] Using pool credential: type={} name={:?} uuid={}",
                cred.provider_type,
                cred.name,
                &cred.uuid[..8]
            ),
        );

        // 启动 Flow 捕获

        // 使用凭证的实际 provider_type（支持自定义 Provider）
        // 对于自定义 Provider ID，凭证的 provider_type 已通过数据库查询正确设置
        let _provider_type = cred.provider_type;

        // 从凭证名称中提取 Provider 显示名称
        // 凭证名称格式：Some("[降级] DeepSeek") 或 Some("DeepSeek")
        let _provider_display_name = cred.name.as_ref().map(|name| {
            // 去掉 "[降级] " 前缀
            if name.starts_with("[降级] ") {
                &name[9..] // "[降级] " 是 9 个字节
            } else {
                name.as_str()
            }
        });

        // 检查是否需要拦截请求
        // **Validates: Requirements 2.1, 2.3, 2.5**

        let response = call_provider_anthropic(&state, &cred, &request, None).await;

        // 记录请求统计
        let is_success = response.status().is_success();
        let status = if is_success {
            proxycast_infra::telemetry::RequestStatus::Success
        } else {
            proxycast_infra::telemetry::RequestStatus::Failed
        };
        record_request_telemetry(&state, &ctx, status, None);

        // 估算 Token 使用量
        let estimated_input_tokens = request
            .messages
            .iter()
            .map(|m| {
                let content_len = match &m.content {
                    serde_json::Value::String(s) => s.len(),
                    serde_json::Value::Array(arr) => arr
                        .iter()
                        .filter_map(|v| v.get("text").and_then(|t| t.as_str()))
                        .map(|s| s.len())
                        .sum(),
                    _ => 0,
                };
                content_len / 4
            })
            .sum::<usize>() as u32;
        let estimated_output_tokens = if is_success { 100u32 } else { 0u32 };

        if is_success {
            record_token_usage(
                &state,
                &ctx,
                Some(estimated_input_tokens),
                Some(estimated_output_tokens),
            );
        }

        // 完成 Flow 捕获并检查响应拦截
        // **Validates: Requirements 2.1, 2.5**

        return response;
    }

    // 回退到旧的单凭证模式（仅当选择的 Provider 是 Kiro 时）
    // 如果选择的 Provider 不是 Kiro，且凭证池中没有找到凭证，返回错误
    // **Validates: Requirements 3.2**
    if selected_provider.to_lowercase() != "kiro" {
        state.logs.write().await.add(
            "error",
            &format!(
                "[ROUTE] No pool credential found for '{selected_provider}' (client_type={client_type}), and legacy mode only supports Kiro"
            ),
        );
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "type": "error",
                "error": {
                    "type": "no_credential_error",
                    "message": format!("没有找到可用的 '{}' 凭证。请在凭证池中添加对应的凭证。", selected_provider)
                }
            })),
        )
            .into_response();
    }

    state.logs.write().await.add(
        "debug",
        &format!("[ROUTE] No pool credential found for '{selected_provider}', using legacy mode"),
    );

    // 启动 Flow 捕获（legacy mode）

    // 使用实际的 provider ID 构建 Flow Metadata
    let _provider_type = selected_provider
        .parse::<ProviderType>()
        .unwrap_or(ProviderType::OpenAI);

    // 检查是否需要拦截请求（legacy mode）
    // **Validates: Requirements 2.1, 2.3, 2.5**

    // 检查是否需要刷新 token（无 token 或即将过期）
    {
        let _guard = state.kiro_refresh_lock.lock().await;
        let mut kiro = state.kiro.write().await;
        let needs_refresh =
            kiro.credentials.access_token.is_none() || kiro.is_token_expiring_soon();
        if needs_refresh {
            state.logs.write().await.add(
                "info",
                "[AUTH] No access token or token expiring soon, attempting refresh...",
            );
            if let Err(e) = kiro.refresh_token().await {
                state
                    .logs
                    .write()
                    .await
                    .add("error", &format!("[AUTH] Token refresh failed: {e}"));
                // 标记 Flow 失败
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({"error": {"message": format!("Token refresh failed: {e}")}})),
                )
                    .into_response();
            }
            state
                .logs
                .write()
                .await
                .add("info", "[AUTH] Token refreshed successfully");
        }
    }

    // 转换为 OpenAI 格式
    let openai_request = convert_anthropic_to_openai(&request);

    // 记录转换后的请求信息
    state.logs.write().await.add(
        "debug",
        &format!(
            "[CONVERT] OpenAI format: messages={} tools={} stream={}",
            openai_request.messages.len(),
            openai_request.tools.as_ref().map(|t| t.len()).unwrap_or(0),
            openai_request.stream
        ),
    );

    let kiro = state.kiro.read().await;

    match kiro.call_api(&openai_request).await {
        Ok(resp) => {
            let status = resp.status();
            state
                .logs
                .write()
                .await
                .add("info", &format!("[RESP] Upstream status: {status}"));

            if status.is_success() {
                match resp.bytes().await {
                    Ok(bytes) => {
                        // 使用 lossy 转换，避免无效 UTF-8 导致崩溃
                        let body = String::from_utf8_lossy(&bytes).to_string();

                        // 记录原始响应长度
                        state.logs.write().await.add(
                            "debug",
                            &format!("[RESP] Raw body length: {} bytes", bytes.len()),
                        );

                        // 保存原始响应到文件用于调试
                        let request_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
                        state.logs.read().await.log_raw_response(&request_id, &body);
                        state.logs.write().await.add(
                            "debug",
                            &format!("[RESP] Raw response saved to raw_response_{request_id}.txt"),
                        );

                        // 记录响应的前200字符用于调试（减少日志量）
                        let preview: String =
                            body.chars().filter(|c| !c.is_control()).take(200).collect();
                        state
                            .logs
                            .write()
                            .await
                            .add("debug", &format!("[RESP] Body preview: {preview}"));

                        let parsed = parse_cw_response(&body);

                        // 详细记录解析结果
                        state.logs.write().await.add(
                            "info",
                            &format!(
                                "[RESP] Parsed: content_len={}, tool_calls={}, content_preview={}",
                                parsed.content.len(),
                                parsed.tool_calls.len(),
                                parsed.content.chars().take(100).collect::<String>()
                            ),
                        );

                        // 记录 tool calls 详情
                        for (i, tc) in parsed.tool_calls.iter().enumerate() {
                            state.logs.write().await.add(
                                "debug",
                                &format!(
                                    "[RESP] Tool call {}: name={} id={}",
                                    i, tc.function.name, tc.id
                                ),
                            );
                        }

                        // 如果请求流式响应，返回 SSE 格式
                        if request.stream {
                            // 完成 Flow 捕获并检查响应拦截（流式）
                            // **Validates: Requirements 2.1, 2.5**
                            return build_anthropic_stream_response(&request.model, &parsed);
                        }

                        // 完成 Flow 捕获并检查响应拦截（非流式）
                        // **Validates: Requirements 2.1, 2.5**

                        // 非流式响应
                        build_anthropic_response(&request.model, &parsed)
                    }
                    Err(e) => {
                        state
                            .logs
                            .write()
                            .await
                            .add("error", &format!("[ERROR] Response body read failed: {e}"));
                        // 标记 Flow 失败
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({"error": {"message": e.to_string()}})),
                        )
                            .into_response()
                    }
                }
            } else if status.as_u16() == 403 || status.as_u16() == 402 {
                // Token 过期或账户问题，尝试重新加载凭证并刷新
                drop(kiro);
                let _guard = state.kiro_refresh_lock.lock().await;
                let mut kiro = state.kiro.write().await;
                state.logs.write().await.add(
                    "warn",
                    &format!(
                        "[AUTH] Got {}, reloading credentials and attempting token refresh...",
                        status.as_u16()
                    ),
                );

                // 先重新加载凭证文件（可能用户换了账户）
                if let Err(e) = kiro.load_credentials().await {
                    state.logs.write().await.add(
                        "error",
                        &format!("[AUTH] Failed to reload credentials: {e}"),
                    );
                }

                match kiro.refresh_token().await {
                    Ok(_) => {
                        state.logs.write().await.add(
                            "info",
                            "[AUTH] Token refreshed successfully, retrying request...",
                        );
                        drop(kiro);
                        let kiro = state.kiro.read().await;
                        match kiro.call_api(&openai_request).await {
                            Ok(retry_resp) => {
                                let retry_status = retry_resp.status();
                                state.logs.write().await.add(
                                    "info",
                                    &format!("[RETRY] Response status: {retry_status}"),
                                );
                                if retry_resp.status().is_success() {
                                    match retry_resp.bytes().await {
                                        Ok(bytes) => {
                                            let body = String::from_utf8_lossy(&bytes).to_string();
                                            let parsed = parse_cw_response(&body);
                                            state.logs.write().await.add(
                                                "info",
                                                &format!(
                                                "[RETRY] Success: content_len={}, tool_calls={}",
                                                parsed.content.len(), parsed.tool_calls.len()
                                            ),
                                            );
                                            // 完成 Flow 捕获并检查响应拦截（重试成功）
                                            // **Validates: Requirements 2.1, 2.5**
                                            if request.stream {
                                                return build_anthropic_stream_response(
                                                    &request.model,
                                                    &parsed,
                                                );
                                            }
                                            return build_anthropic_response(
                                                &request.model,
                                                &parsed,
                                            );
                                        }
                                        Err(e) => {
                                            state.logs.write().await.add(
                                                "error",
                                                &format!("[RETRY] Body read failed: {e}"),
                                            );
                                            // 标记 Flow 失败
                                            return (
                                                StatusCode::INTERNAL_SERVER_ERROR,
                                                Json(serde_json::json!({"error": {"message": e.to_string()}})),
                                            )
                                                .into_response();
                                        }
                                    }
                                }
                                let body = retry_resp
                                    .bytes()
                                    .await
                                    .map(|b| String::from_utf8_lossy(&b).to_string())
                                    .unwrap_or_default();
                                state.logs.write().await.add(
                                    "error",
                                    &format!(
                                        "[RETRY] Failed with status {retry_status}: {}",
                                        safe_truncate(&body, 500)
                                    ),
                                );
                                // 标记 Flow 失败（重试失败）
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(serde_json::json!({"error": {"message": format!("Retry failed: {}", body)}})),
                                )
                                    .into_response()
                            }
                            Err(e) => {
                                state
                                    .logs
                                    .write()
                                    .await
                                    .add("error", &format!("[RETRY] Request failed: {e}"));
                                // 标记 Flow 失败
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(serde_json::json!({"error": {"message": e.to_string()}})),
                                )
                                    .into_response()
                            }
                        }
                    }
                    Err(e) => {
                        state
                            .logs
                            .write()
                            .await
                            .add("error", &format!("[AUTH] Token refresh failed: {e}"));
                        // 标记 Flow 失败
                        (
                            StatusCode::UNAUTHORIZED,
                            Json(serde_json::json!({"error": {"message": format!("Token refresh failed: {e}")}})),
                        )
                            .into_response()
                    }
                }
            } else {
                let body = resp.text().await.unwrap_or_default();
                state.logs.write().await.add(
                    "error",
                    &format!(
                        "[ERROR] Upstream error HTTP {}: {}",
                        status,
                        safe_truncate(&body, 500)
                    ),
                );
                // 标记 Flow 失败
                (
                    StatusCode::from_u16(status.as_u16())
                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                    Json(
                        serde_json::json!({"error": {"message": format!("Upstream error: {}", body)}}),
                    ),
                )
                    .into_response()
            }
        }
        Err(e) => {
            // 详细记录网络/连接错误
            let error_details = format!("{e:?}");
            state
                .logs
                .write()
                .await
                .add("error", &format!("[ERROR] Kiro API call failed: {e}"));
            state.logs.write().await.add(
                "debug",
                &format!("[ERROR] Full error details: {error_details}"),
            );
            // 标记 Flow 失败
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": {"message": e.to_string()}})),
            )
                .into_response()
        }
    }
}

// ============================================================================
// 流式传输辅助函数
// ============================================================================

/// 获取目标流式格式
///
/// 根据请求路径确定目标流式格式。
///
/// # 参数
/// - `path`: 请求路径
///
/// # 返回
/// 目标流式格式
fn get_target_stream_format(path: &str) -> StreamingFormat {
    if path.contains("/v1/messages") {
        // Anthropic 格式端点
        StreamingFormat::AnthropicSse
    } else {
        // OpenAI 格式端点
        StreamingFormat::OpenAiSse
    }
}

/// 检查是否应该使用真正的流式传输
///
/// 根据凭证类型和配置决定是否使用真正的流式传输。
/// 目前，只有当 Provider 实现了 StreamingProvider trait 时才返回 true。
///
/// # 参数
/// - `credential`: 凭证信息
///
/// # 返回
/// 是否应该使用真正的流式传输
///
/// # 注意
/// 当前所有 Provider 都返回 false，因为 StreamingProvider trait 尚未实现。
/// 一旦任务 6 完成，此函数将根据凭证类型返回适当的值。
fn should_use_true_streaming(
    credential: &proxycast_core::models::provider_pool_model::ProviderCredential,
) -> bool {
    use proxycast_core::models::provider_pool_model::CredentialData;

    // TODO: 当 StreamingProvider trait 实现后，根据凭证类型返回 true
    // 目前所有 Provider 都使用伪流式模式
    match &credential.credential {
        // Kiro/CodeWhisperer - 需要实现 StreamingProvider
        CredentialData::KiroOAuth { .. } => false,
        // Claude - 需要实现 StreamingProvider
        CredentialData::ClaudeKey { .. } => false,
        // OpenAI - 需要实现 StreamingProvider
        CredentialData::OpenAIKey { .. } => false,
        // Antigravity - 需要实现 StreamingProvider
        CredentialData::AntigravityOAuth { .. } => false,
        // 其他类型暂不支持流式
        _ => false,
    }
}

/// 构建流式错误响应
///
/// 将错误转换为 SSE 格式的错误事件。
///
/// # 参数
/// - `error_type`: 错误类型
/// - `message`: 错误消息
/// - `target_format`: 目标流式格式
///
/// # 返回
/// SSE 格式的错误响应
///
/// # 需求覆盖
/// - 需求 5.3: 流中发生错误时发送错误事件并优雅关闭流
fn build_stream_error_response(
    error_type: &str,
    message: &str,
    target_format: StreamingFormat,
) -> Response {
    let error_event = match target_format {
        StreamingFormat::AnthropicSse => {
            format!(
                "event: error\ndata: {}\n\n",
                serde_json::json!({
                    "type": "error",
                    "error": {
                        "type": error_type,
                        "message": message
                    }
                })
            )
        }
        // TODO: 任务 6 完成后，添加 GeminiStream 分支
        StreamingFormat::OpenAiSse => {
            format!(
                "data: {}\n\n",
                serde_json::json!({
                    "error": {
                        "type": error_type,
                        "message": message
                    }
                })
            )
        }
        StreamingFormat::AwsEventStream => {
            // AWS Event Stream 格式的错误（不太可能作为目标格式）
            format!(
                "data: {}\n\n",
                serde_json::json!({
                    "error": {
                        "type": error_type,
                        "message": message
                    }
                })
            )
        }
    };

    Response::builder()
        .status(StatusCode::OK) // SSE 错误仍然返回 200
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .body(Body::from(error_event))
        .unwrap_or_else(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": {"message": "Failed to build error response"}})),
            )
                .into_response()
        })
}

/// 将 OpenAI 格式请求转换为 Anthropic 格式
fn convert_openai_to_anthropic(request: &ChatCompletionRequest) -> serde_json::Value {
    let mut messages = Vec::new();
    let mut system_prompt = None;

    for msg in &request.messages {
        if msg.role == "system" {
            // 提取 system prompt
            if let Some(content) = &msg.content {
                system_prompt = Some(match content {
                    proxycast_core::models::openai::MessageContent::Text(s) => s.clone(),
                    proxycast_core::models::openai::MessageContent::Parts(parts) => parts
                        .iter()
                        .filter_map(|p| {
                            if let proxycast_core::models::openai::ContentPart::Text { text } = p {
                                Some(text.clone())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                });
            }
        } else {
            // 转换其他消息
            let content = match &msg.content {
                Some(c) => match c {
                    proxycast_core::models::openai::MessageContent::Text(s) => s.clone(),
                    proxycast_core::models::openai::MessageContent::Parts(parts) => parts
                        .iter()
                        .filter_map(|p| {
                            if let proxycast_core::models::openai::ContentPart::Text { text } = p {
                                Some(text.clone())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                },
                None => String::new(),
            };

            messages.push(serde_json::json!({
                "role": msg.role,
                "content": content
            }));
        }
    }

    let mut result = serde_json::json!({
        "model": request.model,
        "messages": messages,
        "max_tokens": request.max_tokens.unwrap_or(4096),
        "stream": request.stream
    });

    if let Some(system) = system_prompt {
        result["system"] = serde_json::Value::String(system);
    }

    if let Some(temp) = request.temperature {
        result["temperature"] = serde_json::Value::Number(
            serde_json::Number::from_f64(temp as f64).unwrap_or(serde_json::Number::from(1)),
        );
    }

    result
}

/// 将 Anthropic 响应转换为 OpenAI 格式
fn convert_anthropic_response_to_openai(anthropic_resp: &serde_json::Value, model: &str) -> String {
    let content = anthropic_resp["content"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["text"].as_str())
        .unwrap_or("");

    let usage = serde_json::json!({
        "prompt_tokens": anthropic_resp["usage"]["input_tokens"].as_u64().unwrap_or(0),
        "completion_tokens": anthropic_resp["usage"]["output_tokens"].as_u64().unwrap_or(0),
        "total_tokens": anthropic_resp["usage"]["input_tokens"].as_u64().unwrap_or(0)
            + anthropic_resp["usage"]["output_tokens"].as_u64().unwrap_or(0)
    });

    let openai_resp = serde_json::json!({
        "id": anthropic_resp["id"].as_str().unwrap_or("chatcmpl-unknown"),
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": content
            },
            "finish_reason": match anthropic_resp["stop_reason"].as_str() {
                Some("end_turn") => "stop",
                Some("max_tokens") => "length",
                Some("tool_use") => "tool_calls",
                _ => "stop"
            }
        }],
        "usage": usage
    });

    serde_json::to_string(&openai_resp).unwrap_or_default()
}
