use agent_protocol::action_required::ActionRequiredScope;
use futures::future::BoxFuture;
use serde_json::Value;
use std::fmt;
use std::time::Duration;

pub use tool_runtime::request_user_input::{
    build_requested_schema, extract_response, resolve_request_prompt, RequestUserInputCallback,
    RequestUserInputOption, RequestUserInputQuestion, RequestUserInputRequest,
    DEFAULT_REQUEST_USER_INPUT_TIMEOUT_SECS, REQUEST_USER_INPUT_QUESTIONS_SCHEMA_KEY,
};

#[derive(Debug, Clone, PartialEq)]
pub struct RequestUserInputRunRequest {
    pub request: RequestUserInputRequest,
    pub is_blocking: bool,
    pub scope: Option<ActionRequiredScope>,
    pub timeout: Duration,
}

impl RequestUserInputRunRequest {
    pub fn new(
        request: RequestUserInputRequest,
        is_blocking: bool,
        scope: Option<ActionRequiredScope>,
        timeout: Duration,
    ) -> Self {
        Self {
            request,
            is_blocking,
            scope,
            timeout,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RequestUserInputAction {
    pub prompt: String,
    pub requested_schema: Value,
    pub is_blocking: bool,
    pub auto_resolution_ms: Option<u64>,
    pub scope: Option<ActionRequiredScope>,
    pub timeout: Duration,
}

pub trait RequestUserInputGateway: Send + Sync {
    fn request_user_input<'a>(
        &'a self,
        action: RequestUserInputAction,
    ) -> BoxFuture<'a, anyhow::Result<Value>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestUserInputError {
    prompt: String,
    message: String,
}

impl RequestUserInputError {
    pub fn new(prompt: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            message: message.into(),
        }
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for RequestUserInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for RequestUserInputError {}

pub async fn run_request_user_input<G>(
    gateway: &G,
    run_request: RequestUserInputRunRequest,
) -> Result<Option<Value>, RequestUserInputError>
where
    G: RequestUserInputGateway + ?Sized,
{
    let prompt = resolve_request_prompt(&run_request.request);
    let action = RequestUserInputAction {
        prompt: prompt.clone(),
        requested_schema: build_requested_schema(&run_request.request),
        is_blocking: run_request.is_blocking,
        auto_resolution_ms: run_request.request.auto_resolution_ms,
        scope: run_request.scope,
        timeout: run_request.timeout,
    };
    let user_data = gateway
        .request_user_input(action)
        .await
        .map_err(|err| RequestUserInputError::new(prompt, err.to_string()))?;

    Ok(extract_response(&run_request.request, &user_data))
}

#[cfg(test)]
mod tests {
    use super::{
        build_requested_schema, extract_response, run_request_user_input, RequestUserInputAction,
        RequestUserInputGateway, RequestUserInputOption, RequestUserInputQuestion,
        RequestUserInputRequest, RequestUserInputRunRequest,
        REQUEST_USER_INPUT_QUESTIONS_SCHEMA_KEY,
    };
    use agent_protocol::action_required::ActionRequiredScope;
    use futures::future::BoxFuture;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[derive(Clone)]
    struct CapturingGateway {
        actions: Arc<Mutex<Vec<RequestUserInputAction>>>,
        response: serde_json::Value,
    }

    impl RequestUserInputGateway for CapturingGateway {
        fn request_user_input<'a>(
            &'a self,
            action: RequestUserInputAction,
        ) -> BoxFuture<'a, anyhow::Result<serde_json::Value>> {
            Box::pin(async move {
                self.actions.lock().expect("actions lock").push(action);
                Ok(self.response.clone())
            })
        }
    }

    #[test]
    fn build_requested_schema_embeds_questions_extension() {
        let request = RequestUserInputRequest {
            questions: vec![RequestUserInputQuestion {
                id: Some("primary_color".to_string()),
                question: "你希望主色调是什么？".to_string(),
                header: Some("主色调".to_string()),
                options: vec![
                    RequestUserInputOption {
                        value: "blue-purple".to_string(),
                        label: Some("蓝紫".to_string()),
                        description: Some("冷色科技感".to_string()),
                        preview: None,
                    },
                    RequestUserInputOption {
                        value: "cyber-green".to_string(),
                        label: Some("赛博绿".to_string()),
                        description: Some("高亮未来感".to_string()),
                        preview: None,
                    },
                ],
                multi_select: false,
            }],
            auto_resolution_ms: None,
        };

        let schema = build_requested_schema(&request);
        assert_eq!(
            schema
                .get(REQUEST_USER_INPUT_QUESTIONS_SCHEMA_KEY)
                .and_then(|value| value.as_array())
                .map(|value| value.len()),
            Some(1)
        );
        assert_eq!(
            schema["properties"]["answer"]["enum"],
            serde_json::json!(["蓝紫", "赛博绿"])
        );
    }

    #[test]
    fn extract_response_normalizes_question_answers() {
        let request = RequestUserInputRequest {
            questions: vec![
                RequestUserInputQuestion::new("第一问"),
                RequestUserInputQuestion {
                    id: Some("mode".to_string()),
                    question: "第二问".to_string(),
                    header: Some("mode".to_string()),
                    options: vec![
                        RequestUserInputOption::with_label("auto", "自动执行"),
                        RequestUserInputOption::with_label("confirm", "确认后执行"),
                    ],
                    multi_select: false,
                },
            ],
            auto_resolution_ms: None,
        };

        let response = extract_response(
            &request,
            &serde_json::json!({
                "question_1": "先看结构",
                "mode": "确认后执行"
            }),
        )
        .expect("expected normalized response");

        assert_eq!(
            response,
            serde_json::json!({
                "answers": {
                    "第一问": "先看结构",
                    "第二问": "confirm"
                }
            })
        );
    }

    #[test]
    fn extract_response_normalizes_multi_select_answers() {
        let request = RequestUserInputRequest {
            questions: vec![RequestUserInputQuestion {
                id: Some("skills".to_string()),
                question: "请选择能力".to_string(),
                header: Some("skills".to_string()),
                options: vec![
                    RequestUserInputOption::with_label("analysis", "分析"),
                    RequestUserInputOption::with_label("coding", "编码"),
                ],
                multi_select: true,
            }],
            auto_resolution_ms: None,
        };

        let response = extract_response(
            &request,
            &serde_json::json!({
                "answer": ["分析", "编码"]
            }),
        )
        .expect("expected normalized response");

        assert_eq!(
            response,
            serde_json::json!({
                "answer": "analysis, coding",
                "answers": {
                    "请选择能力": "analysis, coding"
                }
            })
        );
    }

    #[test]
    fn run_request_user_input_builds_action_and_normalizes_response() {
        let actions = Arc::new(Mutex::new(Vec::new()));
        let gateway = CapturingGateway {
            actions: Arc::clone(&actions),
            response: serde_json::json!({
                "answer": "确认后执行"
            }),
        };
        let request = RequestUserInputRequest {
            questions: vec![RequestUserInputQuestion {
                id: Some("mode".to_string()),
                question: "请选择执行模式".to_string(),
                header: Some("mode".to_string()),
                options: vec![
                    RequestUserInputOption::with_label("auto", "自动执行"),
                    RequestUserInputOption::with_label("confirm", "确认后执行"),
                ],
                multi_select: false,
            }],
            auto_resolution_ms: None,
        };
        let scope = Some(ActionRequiredScope {
            session_id: Some("session-1".to_string()),
            thread_id: Some("thread-1".to_string()),
            turn_id: Some("turn-1".to_string()),
        });

        let response = futures::executor::block_on(run_request_user_input(
            &gateway,
            RequestUserInputRunRequest::new(request, true, scope.clone(), Duration::from_secs(5)),
        ))
        .expect("request should run")
        .expect("response should normalize");

        assert_eq!(
            response,
            serde_json::json!({
                "answer": "confirm",
                "answers": {
                    "请选择执行模式": "confirm"
                }
            })
        );

        let actions = actions.lock().expect("actions lock");
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].prompt, "请选择执行模式");
        assert!(actions[0].is_blocking);
        assert_eq!(actions[0].scope, scope);
        assert_eq!(actions[0].timeout, Duration::from_secs(5));
        assert_eq!(
            actions[0].requested_schema["properties"]["answer"]["enum"],
            serde_json::json!(["自动执行", "确认后执行"])
        );
    }
}
