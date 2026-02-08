//! 录音服务桥接层
//!
//! 纯逻辑已迁移到 `proxycast-services` crate，
//! 本模块保留兼容导出。

pub use proxycast_services::voice_recording_service::{
    create_recording_service_state, list_audio_devices, AudioDeviceInfo, RecordingCommand,
    RecordingResponse, RecordingService, RecordingServiceState,
};
