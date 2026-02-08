//! 语音输入配置管理（桥接层）
//!
//! 纯逻辑已迁移到 `proxycast-services` crate，
//! 本模块保留兼容导出。

pub use proxycast_services::voice_config_service::{
    asr_provider_name, delete_voice_instruction, get_asr_credential, get_default_asr_credential,
    get_enabled_asr_credential_by_provider, get_instruction, get_instructions,
    list_asr_credentials, load_voice_config, resolve_output_mode, save_voice_config,
    save_voice_instruction,
};
