/**
 * Mock for @tauri-apps/api/core
 */

// 模拟的命令处理器
const mockCommands = new Map<string, (...args: any[]) => any>();

// 默认 mock 数据
const defaultMocks: Record<string, any> = {
  // 配置相关
  get_config: () => ({
    server: {
      host: "127.0.0.1",
      port: 8787,
      api_key: "",
      tls: {
        enable: false,
        cert_path: null,
        key_path: null,
      },
    },
    providers: {
      kiro: {
        enabled: false,
        credentials_path: null,
        region: null,
      },
      gemini: {
        enabled: false,
        credentials_path: null,
      },
      qwen: {
        enabled: false,
        credentials_path: null,
      },
      openai: {
        enabled: false,
        api_key: null,
        base_url: null,
      },
      claude: {
        enabled: false,
        api_key: null,
        base_url: null,
      },
    },
    default_provider: "kiro",
    remote_management: {
      allow_remote: false,
      secret_key: null,
      disable_control_panel: false,
    },
    quota_exceeded: {
      switch_project: true,
      switch_preview_model: false,
      cooldown_seconds: 60,
    },
    ampcode: {
      upstream_url: null,
      model_mappings: [],
      restrict_management_to_localhost: true,
    },
    credential_pool: {
      kiro: [],
      gemini: [],
      qwen: [],
      openai: [],
      claude: [],
      gemini_api_keys: [],
      vertex_api_keys: [],
      codex: [],
      iflow: [],
    },
    proxy_url: null,
    minimize_to_tray: false,
    language: "zh",
    experimental: {
      screenshot_chat: {
        enabled: false,
        shortcut: "",
      },
    },
  }),

  save_config: (config: any) => {
    console.log("[Mock] Config saved:", config);
    return { success: true };
  },

  // Provider 相关
  get_providers: () => [],
  get_credentials: () => [],
  get_default_provider: () => "kiro",
  set_default_provider: (args: any) => {
    const provider = args?.provider ?? args;
    console.log("[Mock] Default provider set to:", provider);
    return provider;
  },
  get_available_models: () => [],

  // 服务器相关
  get_server_status: () => ({
    running: false,
    host: "127.0.0.1",
    port: 8787,
    requests: 0,
    uptime_secs: 0,
  }),
  check_server_status: () => ({
    running: false,
    host: "127.0.0.1",
    port: 8787,
    requests: 0,
    uptime_secs: 0,
  }),
  start_server: () => "Server started (mock)",
  stop_server: () => "Server stopped (mock)",

  // 网络相关
  get_network_info: () => ({
    localhost: "127.0.0.1",
    lan_ip: "192.168.1.100",
    all_ips: ["127.0.0.1", "192.168.1.100"],
  }),

  // Agent 相关
  list_agent_sessions: () => [],
  agent_list_sessions: () => [],
  get_agent_process_status: () => ({ running: false }),
  agent_get_process_status: () => ({ running: false }),
  agent_start_process: () => ({ success: true }),
  agent_stop_process: () => ({ success: true }),
  agent_create_session: () => ({ session_id: "mock-session-id" }),
  agent_send_message: () => ({ message_id: "mock-message-id" }),
  agent_get_session: () => ({ session: null }),
  agent_delete_session: () => ({ success: true }),
  agent_get_session_messages: () => [],
  agent_chat_stream: () => ({}),
  agent_terminal_command_response: () => ({}),
  agent_term_scrollback_response: () => ({}),

  // Aster Agent
  aster_agent_init: () => ({ initialized: true, provider_configured: false }),
  aster_agent_status: () => ({
    initialized: false,
    provider_configured: false,
  }),
  aster_agent_configure_provider: () => ({
    initialized: true,
    provider_configured: true,
  }),
  aster_agent_configure_from_pool: () => ({
    initialized: true,
    provider_configured: true,
  }),
  aster_agent_chat_stream: () => ({}),
  aster_agent_stop: () => true,
  aster_session_create: () => "mock-aster-session",
  aster_session_list: () => [],
  aster_session_get: () => ({ id: "mock", messages: [] }),
  aster_agent_confirm: () => ({}),

  // 终端相关
  create_terminal_session: () => ({ uuid: "mock-terminal-uuid" }),
  terminal_create_session: () => ({ uuid: "mock-terminal-uuid" }),
  terminal_write: () => ({}),
  terminal_resize: () => ({}),
  terminal_close: () => ({}),
  read_terminal_output: () => [],
  list_terminal_sessions: () => [],

  // 技能相关
  get_all_skills: () => [],
  get_skills_for_app: () => [],
  get_skill_repos: () => [],
  add_skill_repo: () => ({ success: true }),
  remove_skill_repo: () => ({ success: true }),
  get_installed_proxycast_skills: () => [],
  install_skill_for_app: () => ({ success: true }),
  uninstall_skill_for_app: () => ({ success: true }),
  enable_skill: () => ({ success: true }),
  disable_skill: () => ({ success: true }),

  // 插件相关
  get_plugins_with_ui: () => [],
  get_plugin_status: () => ({
    enabled: true,
    plugin_count: 0,
    plugins_dir: "/mock/plugins",
  }),
  get_plugins: () => [],
  list_installed_plugins: () => [],
  enable_plugin: () => ({ success: true }),
  disable_plugin: () => ({ success: true }),
  reload_plugins: () => ({ success: true }),
  unload_plugin: () => ({ success: true }),
  uninstall_plugin: () => ({ success: true }),
  launch_plugin_ui: () => ({}),

  // 凭证池相关
  get_relay_providers: () => [],
  list_relay_providers: () => [],
  get_pool_overview: () => [],
  get_provider_pool_overview: () => [],
  get_provider_pool_credentials: () => [],
  add_provider_pool_credential: () => ({ success: true }),
  update_provider_pool_credential: () => ({ success: true }),
  delete_provider_pool_credential: () => ({ success: true }),
  toggle_provider_pool_credential: () => ({ success: true }),
  reset_provider_pool_credential: () => ({ success: true }),
  reset_provider_pool_health: () => ({ success: true }),
  check_provider_pool_credential_health: () => ({ healthy: false }),
  check_provider_pool_type_health: () => ({ healthy: false }),

  // API Key Provider 相关
  get_api_key_providers: () => [],
  get_api_key_provider: () => null,
  add_custom_api_key_provider: () => ({ success: true }),
  update_api_key_provider: () => ({ success: true }),
  delete_custom_api_key_provider: () => ({ success: true }),
  add_api_key: () => ({ success: true }),
  delete_api_key: () => ({ success: true }),
  toggle_api_key: () => ({ success: true }),
  update_api_key_alias: () => ({ success: true }),
  get_next_api_key: () => null,
  record_api_key_usage: () => ({}),
  record_api_key_error: () => ({}),
  get_provider_ui_state: () => null,
  set_provider_ui_state: () => ({}),
  update_provider_sort_orders: () => ({ success: true }),
  export_api_key_providers: () => ({ config: "{}" }),
  import_api_key_providers: () => ({ success: true }),
  get_legacy_api_key_credentials: () => [],
  migrate_legacy_api_key_credentials: () => ({ success: true }),
  delete_legacy_api_key_credential: () => ({ success: true }),
  get_local_kiro_credential_uuid: () => null,

  // OAuth 凭证相关
  add_kiro_oauth_credential: () => ({ success: true }),
  add_kiro_from_json: () => ({ success: true }),
  add_gemini_oauth_credential: () => ({ success: true }),
  add_qwen_oauth_credential: () => ({ success: true }),
  add_openai_key_credential: () => ({ success: true }),
  add_claude_key_credential: () => ({ success: true }),
  add_gemini_api_key_credential: () => ({ success: true }),
  add_antigravity_oauth_credential: () => ({ success: true }),
  add_codex_oauth_credential: () => ({ success: true }),
  add_claude_oauth_credential: () => ({ success: true }),
  add_iflow_oauth_credential: () => ({ success: true }),
  add_iflow_cookie_credential: () => ({ success: true }),
  start_kiro_builder_id_login: () => ({ success: true }),
  poll_kiro_builder_id_auth: () => ({ status: "pending" }),
  cancel_kiro_builder_id_login: () => ({ success: true }),
  add_kiro_from_builder_id_auth: () => ({ success: true }),
  start_kiro_social_auth_login: () => ({ success: true }),
  exchange_kiro_social_auth_token: () => ({ success: true }),
  cancel_kiro_social_auth_login: () => ({ success: true }),
  start_kiro_social_auth_callback_server: () => ({ success: true }),
  refresh_pool_credential_token: () => ({ success: true }),
  get_pool_credential_oauth_status: () => ({ status: "unknown" }),
  migrate_private_config_to_pool: () => ({ success: true }),
  get_credential_health: () => ({ healthy: false }),
  get_all_credential_health: () => [],
  get_kiro_credential_fingerprint: () => ({ fingerprint: "" }),
  switch_kiro_to_local: () => ({ success: true }),

  // Playwright 相关
  check_playwright_available: () => ({ available: false }),
  install_playwright: () => ({ success: true }),
  start_kiro_playwright_login: () => ({ success: true }),
  cancel_kiro_playwright_login: () => ({ success: true }),

  // 连接相关
  list_connections: () => [],
  connection_list: () => [],
  get_oauth_url: () => ({ url: "https://example.com/oauth" }),
  save_oauth_credential: () => ({ success: true }),
  get_oauth_credentials: () => [],
  get_all_oauth_credentials: () => [],
  reload_oauth_credentials: () => ({ success: true }),
  refresh_oauth_token: () => ({ success: true }),
  get_oauth_env_variables: () => [],
  get_oauth_token_file_hash: () => ({ hash: "" }),
  check_and_reload_oauth_credentials: () => ({
    changed: false,
    new_hash: "",
    reloaded: false,
  }),

  // 模型相关
  get_model_registry: () => [],
  refresh_model_registry: () => ({ success: true }),
  search_models: () => [],
  get_all_provider_models: () => ({}),
  get_model_preferences: () => [],
  toggle_model_favorite: () => ({ success: true }),
  hide_model: () => ({ success: true }),
  record_model_usage: () => ({}),
  get_model_sync_state: () => ({ syncing: false, last_sync_at: null }),
  get_models_for_provider: () => [],
  get_models_by_tier: () => [],
  get_provider_alias_config: () => ({ alias: {} }),
  get_all_alias_configs: () => ({}),

  // Orchestrator 相关
  init_orchestrator: () => ({}),
  get_orchestrator_config: () => ({ config: {} }),
  update_orchestrator_config: () => ({ success: true }),
  get_pool_stats: () => ({ stats: {} }),
  get_tier_models: () => [],
  get_all_models: () => [],
  update_orchestrator_credentials: () => ({ success: true }),
  add_orchestrator_credential: () => ({ success: true }),
  remove_orchestrator_credential: () => ({ success: true }),
  mark_credential_unhealthy: () => ({ success: true }),
  mark_credential_healthy: () => ({ success: true }),
  update_credential_load: () => ({ success: true }),
  select_model: () => ({ model: "" }),
  quick_select_model: () => ({ model: "" }),
  select_model_for_task: () => ({ model: "" }),
  list_strategies: () => [],
  list_service_tiers: () => [],
  list_task_hints: () => [],

  // MCP 相关
  get_mcp_servers: () => [],
  add_mcp_server: () => ({ success: true }),
  update_mcp_server: () => ({ success: true }),
  delete_mcp_server: () => ({ success: true }),
  toggle_mcp_server: () => ({ success: true }),
  import_mcp_from_app: () => ({ success: true }),
  sync_all_mcp_to_live: () => ({ success: true }),
  sync_from_external_config: () => ({ success: true }),

  // Switch Provider 相关
  get_switch_providers: () => [],
  add_switch_provider: () => ({ success: true }),
  delete_switch_provider: () => ({ success: true }),
  update_switch_provider: () => ({ success: true }),
  get_current_switch_provider: () => null,
  read_live_provider_settings: () => ({}),

  // Flow Monitor 相关
  subscribe_flow_events: () => ({ success: true }),
  query_flows: () => ({ flows: [], total: 0 }),
  query_flows_with_expression: () => ({ flows: [], total: 0 }),
  get_flow_detail: () => ({ flow: null }),
  search_flows: () => ({ flows: [] }),
  get_flow_stats: () => ({ stats: {} }),
  update_flow_annotations: () => ({ success: true }),
  cleanup_flows: () => ({ deleted_count: 0 }),
  get_recent_flows: () => [],
  toggle_flow_starred: () => ({ success: true }),
  get_all_flow_tags: () => [],
  delete_flow: () => ({ success: true }),
  delete_flows: () => ({ success: true }),
  get_flow_monitor_debug_info: () => ({ info: {} }),
  create_test_flows: () => ({ created_count: 0 }),
  get_enhanced_stats: () => ({ stats: {} }),
  get_request_trend: () => ({ trend: [] }),
  get_token_distribution: () => ({ distribution: {} }),
  get_latency_histogram: () => ({ histogram: [] }),
  export_stats_report: () => ({ report: "" }),
  get_threshold_config: () => ({ config: {} }),
  update_threshold_config: () => ({ success: true }),
  get_request_rate: () => ({ rate: 0 }),
  set_rate_window: () => ({ success: true }),

  // 系统信息相关
  subscribe_sysinfo: () => ({ success: true }),
  unsubscribe_sysinfo: () => ({ success: true }),

  // Session 相关
  update_session: () => ({ success: true }),
  add_flow_to_session: () => ({ success: true }),
  remove_flow_from_session: () => ({ success: true }),
  unarchive_session: () => ({ success: true }),
  archive_session: () => ({ success: true }),
  delete_session: () => ({ success: true }),

  // Bookmark 相关
  remove_bookmark: () => ({ success: true }),

  // Intercept 相关
  intercept_config_set: () => ({ success: true }),
  intercept_continue: () => ({ success: true }),
  intercept_cancel: () => ({ success: true }),

  // Quick Filter 相关
  delete_quick_filter: () => ({ success: true }),

  // Telemetry 相关
  get_request_logs: () => ({ logs: [] }),
  get_request_log_detail: () => ({ log: null }),
  clear_request_logs: () => ({ success: true }),
  get_stats_summary: () => ({ summary: {} }),
  get_stats_by_provider: () => ({ stats: [] }),
  get_stats_by_model: () => ({ stats: [] }),
  get_token_summary: () => ({ summary: {} }),
  get_token_stats_by_provider: () => ({ stats: [] }),
  get_token_stats_by_model: () => ({ stats: [] }),
  get_token_stats_by_day: () => ({ stats: [] }),

  // Routes 相关
  get_available_routes: () => ({ routes: [] }),
  get_route_curl_examples: () => ({ examples: [] }),

  // Prompts 相关
  get_prompts: () => [],
  upsert_prompt: () => ({ success: true }),
  add_prompt: () => ({ success: true }),
  update_prompt: () => ({ success: true }),
  delete_prompt: () => ({ success: true }),
  enable_prompt: () => ({ success: true }),
  import_prompt_from_file: () => ({ success: true }),
  get_current_prompt_file_content: () => ({ content: "" }),
  auto_import_prompt: () => ({ success: true }),
  switch_prompt: () => ({ success: true }),

  // Browser Interceptor 相关
  get_browser_interceptor_state: () => ({ running: false }),
  start_browser_interceptor: () => ({ success: true }),
  stop_browser_interceptor: () => ({ success: true }),
  restore_normal_browser_behavior: () => ({ success: true }),
  temporary_disable_interceptor: () => ({ success: true }),
  get_intercepted_urls: () => ({ urls: [] }),
  get_interceptor_history: () => ({ history: [] }),
  copy_intercepted_url_to_clipboard: () => ({ success: true }),
  open_url_in_fingerprint_browser: () => ({ success: true }),
  dismiss_intercepted_url: () => ({ success: true }),
  update_browser_interceptor_config: () => ({ success: true }),
  get_default_browser_interceptor_config: () => ({ config: {} }),
  validate_browser_interceptor_config: () => ({ valid: true }),
  is_browser_interceptor_running: () => ({ running: false }),
  get_browser_interceptor_statistics: () => ({ stats: {} }),

  // Notification 相关
  show_notification: () => ({}),
  show_url_intercept_notification: () => ({}),
  show_status_notification: () => ({}),

  // Window 相关
  get_window_size: () => ({ width: 1280, height: 800 }),
  set_window_size: () => ({}),
  get_window_size_options: () => ({ options: [] }),
  set_window_size_by_option: () => ({}),
  toggle_fullscreen: () => ({}),
  is_fullscreen: () => ({ fullscreen: false }),
  resize_for_flow_monitor: () => ({}),
  restore_window_size: () => ({}),
  toggle_window_size: () => ({}),
  center_window: () => ({}),

  // Usage 相关
  get_kiro_usage: () => ({ usage: {} }),

  // Resilience 相关
  get_retry_config: () => ({ config: {} }),
  update_retry_config: () => ({ success: true }),
  get_failover_config: () => ({ config: {} }),
  update_failover_config: () => ({ success: true }),
  get_switch_log: () => ({ logs: [] }),
  clear_switch_log: () => ({ success: true }),

  // Machine ID 相关
  get_current_machine_id: () => ({ machine_id: "" }),
  set_machine_id: () => ({ success: true }),
  generate_random_machine_id: () => ({ machine_id: "" }),
  validate_machine_id: () => ({ valid: true }),
  check_admin_privileges: () => ({ is_admin: false }),
  get_os_type: () => ({ os_type: "linux" }),
  backup_machine_id_to_file: () => ({ success: true }),
  restore_machine_id_from_file: () => ({ success: true }),
  format_machine_id: () => ({ formatted: "" }),
  detect_machine_id_format: () => ({ format: "unknown" }),
  convert_machine_id_format: () => ({ converted: "" }),
  get_machine_id_history: () => ({ history: [] }),
  clear_machine_id_override: () => ({ success: true }),
  copy_machine_id_to_clipboard: () => ({ success: true }),
  paste_machine_id_from_clipboard: () => ({ machine_id: "" }),
  get_system_info: () => ({ info: {} }),

  // Injection 相关
  get_injection_config: () => ({ config: {} }),
  set_injection_enabled: () => ({ success: true }),
  add_injection_rule: () => ({ success: true }),
  remove_injection_rule: () => ({ success: true }),
  update_injection_rule: () => ({ success: true }),
  get_injection_rules: () => ({ rules: [] }),

  // OAuth 登录相关
  start_antigravity_oauth_login: () => ({ success: true }),
  get_antigravity_auth_url_and_wait: () => ({ url: "" }),
  start_codex_oauth_login: () => ({ success: true }),
  get_codex_auth_url_and_wait: () => ({ url: "" }),
  start_claude_oauth_login: () => ({ success: true }),
  get_claude_oauth_auth_url_and_wait: () => ({ url: "" }),
  claude_oauth_with_cookie: () => ({ success: true }),
  start_qwen_device_code_login: () => ({ success: true }),
  get_qwen_device_code_and_wait: () => ({ code: "" }),
  start_iflow_oauth_login: () => ({ success: true }),
  get_iflow_auth_url_and_wait: () => ({ url: "" }),
  start_gemini_oauth_login: () => ({ success: true }),
  get_gemini_auth_url_and_wait: () => ({ url: "" }),
  exchange_gemini_code: () => ({ success: true }),

  // File System 相关
  reveal_in_finder: () => ({}),
  open_with_default_app: () => ({}),
  delete_file: () => ({ success: true }),
  create_file: () => ({ success: true }),
  create_directory: () => ({ success: true }),
  rename_file: () => ({ success: true }),
  list_dir: (args: any) => ({
    path: args?.path ?? "~",
    parentPath: null,
    entries: [],
    error: null,
  }),

  // Log 相关
  get_logs: () => [],
  clear_logs: () => ({}),

  // Test 相关
  test_api: () => ({ success: true, status: 200, body: "", time_ms: 0 }),

  // Kiro Credentials 相关
  get_kiro_credentials: () => ({ loaded: false }),
  refresh_kiro_token: () => ({ success: true }),
  reload_credentials: () => ({ success: true }),
  get_env_variables: () => [],
  get_token_file_hash: () => ({ hash: "" }),
  check_and_reload_credentials: () => ({
    changed: false,
    new_hash: "",
    reloaded: false,
  }),

  // Gemini Credentials 相关
  get_gemini_credentials: () => ({ loaded: false }),
  reload_gemini_credentials: () => ({ success: true }),
  refresh_gemini_token: () => ({ success: true }),
  get_gemini_env_variables: () => [],
  get_gemini_token_file_hash: () => ({ hash: "" }),
  check_and_reload_gemini_credentials: () => ({
    changed: false,
    new_hash: "",
    reloaded: false,
  }),

  // Qwen Credentials 相关
  get_qwen_credentials: () => ({ loaded: false }),
  reload_qwen_credentials: () => ({ success: true }),
  refresh_qwen_token: () => ({ success: true }),
  get_qwen_env_variables: () => [],
  get_qwen_token_file_hash: () => ({ hash: "" }),
  check_and_reload_qwen_credentials: () => ({
    changed: false,
    new_hash: "",
    reloaded: false,
  }),

  // OpenAI Custom 相关
  get_openai_custom_status: () => ({
    enabled: false,
    has_api_key: false,
    base_url: "",
  }),
  set_openai_custom_config: () => ({ success: true }),

  // Claude Custom 相关
  get_claude_custom_status: () => ({
    enabled: false,
    has_api_key: false,
    base_url: "",
  }),
  set_claude_custom_config: () => ({ success: true }),

  // API Compatibility Check 相关
  check_api_compatibility: () => ({
    provider: "",
    overall_status: "ok",
    checked_at: "",
    results: [],
    warnings: [],
  }),

  // Endpoint Providers 相关
  get_endpoint_providers: () => ({}),
  set_endpoint_provider: () => ({ provider: "" }),

  // Experimental Features 相关
  get_experimental_config: () => ({
    screenshot_chat: { enabled: false, shortcut: "" },
  }),
  save_experimental_config: () => ({}),
  validate_shortcut: () => ({ valid: true }),
  update_screenshot_shortcut: () => ({ success: true }),

  // Screenshot Chat 相关
  send_screenshot_chat: () => ({ success: true }),
  close_screenshot_chat_window: () => ({}),

  // Update 相关
  download_update: () => ({ success: true }),
  skip_update_version: () => ({}),
  close_update_window: () => ({}),
  set_update_check_settings: () => ({ success: true }),
  test_update_window: () => ({}),

  // Auto Fix 相关
  auto_fix_configuration: () => ({ success: true }),

  // Check Config Sync 相关
  check_config_sync_status: () => ({ status: "synced" }),
};

/**
 * Mock invoke function
 */
export async function invoke<T = any>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  console.log(`[Mock] invoke: ${cmd}`, args);

  // 检查是否有自定义 mock
  if (mockCommands.has(cmd)) {
    const handler = mockCommands.get(cmd)!;
    return handler(args);
  }

  // 使用默认 mock
  if (cmd in defaultMocks) {
    return defaultMocks[cmd](args);
  }

  console.warn(`[Mock] Unhandled command: ${cmd}`);
  return undefined as T;
}

/**
 * Register a mock command handler
 */
export function mockCommand(cmd: string, handler: (...args: any[]) => any) {
  mockCommands.set(cmd, handler);
}

/**
 * Clear all mock commands
 */
export function clearMocks() {
  mockCommands.clear();
}

/**
 * Mock convertFileSrc function
 * 在真实 Tauri 环境中，这个函数将本地文件路径转换为可在 webview 中使用的 URL
 * 在 mock 环境中，直接返回原始路径（或 blob URL 如果需要）
 */
export function convertFileSrc(filePath: string, _protocol?: string): string {
  // 在 mock 环境中，返回一个占位符或原始路径
  // 实际图片无法在 web 环境中显示，但不会导致构建错误
  console.log(`[Mock] convertFileSrc: ${filePath}`);
  return filePath;
}

// 导出类型以保持兼容
export type { InvokeOptions } from "@tauri-apps/api/core";
