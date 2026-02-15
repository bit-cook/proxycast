//! ASR 服务端到端测试
//!
//! 测试各个 ASR 服务的连接和基本功能。
//!
//! ## 运行测试
//!
//! 需要设置环境变量：
//!
//! ```bash
//! # 讯飞
//! export XUNFEI_APP_ID=xxx
//! export XUNFEI_API_KEY=xxx
//! export XUNFEI_API_SECRET=xxx
//!
//! # 百度
//! export BAIDU_API_KEY=xxx
//! export BAIDU_SECRET_KEY=xxx
//!
//! # OpenAI
//! export OPENAI_API_KEY=xxx
//!
//! # 运行测试
//! cargo test --package voice-core --test asr_e2e_tests -- --nocapture
//! ```

use voice_core::asr_client::{AsrClient, XunfeiClient};
use voice_core::types::AudioData;

/// 生成测试用的静音音频数据
/// 16kHz, 16-bit, 单声道, 1秒
fn generate_silence_audio(duration_secs: f32) -> AudioData {
    let sample_rate = 16000u32;
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let samples = vec![0i16; num_samples];
    AudioData::new(samples, sample_rate, 1)
}

/// 生成测试用的正弦波音频数据（模拟有声音的音频）
/// 16kHz, 16-bit, 单声道
fn generate_sine_wave_audio(duration_secs: f32, frequency: f32) -> AudioData {
    let sample_rate = 16000u32;
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let samples: Vec<i16> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            let amplitude = 0.3 * i16::MAX as f32;
            (amplitude * (2.0 * std::f32::consts::PI * frequency * t).sin()) as i16
        })
        .collect();
    AudioData::new(samples, sample_rate, 1)
}

mod xunfei {
    use super::*;

    fn get_xunfei_credentials() -> Option<(String, String, String)> {
        let app_id = std::env::var("XUNFEI_APP_ID").ok()?;
        let api_key = std::env::var("XUNFEI_API_KEY").ok()?;
        let api_secret = std::env::var("XUNFEI_API_SECRET").ok()?;
        Some((app_id, api_key, api_secret))
    }

    #[tokio::test]
    async fn test_xunfei_connection() {
        let Some((app_id, api_key, api_secret)) = get_xunfei_credentials() else {
            eprintln!("跳过测试: 未设置讯飞凭证环境变量");
            return;
        };

        let client = XunfeiClient::new(app_id, api_key, api_secret);

        // 使用静音音频测试连接
        let audio = generate_silence_audio(1.0);
        let result = client.transcribe(&audio).await;

        match result {
            Ok(r) => {
                println!("✅ 讯飞连接成功");
                println!("   识别结果: {:?}", r.text);
            }
            Err(e) => {
                panic!("❌ 讯飞连接失败: {e:?}");
            }
        }
    }

    #[tokio::test]
    async fn test_xunfei_transcribe_audio() {
        let Some((app_id, api_key, api_secret)) = get_xunfei_credentials() else {
            eprintln!("跳过测试: 未设置讯飞凭证环境变量");
            return;
        };

        let client = XunfeiClient::new(app_id, api_key, api_secret);

        // 使用正弦波音频测试（模拟有声音）
        let audio = generate_sine_wave_audio(2.0, 440.0);
        let result = client.transcribe(&audio).await;

        match result {
            Ok(r) => {
                println!("✅ 讯飞识别成功");
                println!("   识别结果: {:?}", r.text);
                println!("   语言: {:?}", r.language);
            }
            Err(e) => {
                panic!("❌ 讯飞识别失败: {e:?}");
            }
        }
    }

    #[tokio::test]
    async fn test_xunfei_invalid_credentials() {
        let client = XunfeiClient::new(
            "invalid_app_id".to_string(),
            "invalid_api_key".to_string(),
            "invalid_api_secret".to_string(),
        );

        let audio = generate_silence_audio(1.0);
        let result = client.transcribe(&audio).await;

        // 应该返回错误
        assert!(result.is_err(), "无效凭证应该返回错误");
        println!("✅ 无效凭证正确返回错误: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_xunfei_short_audio() {
        let Some((app_id, api_key, api_secret)) = get_xunfei_credentials() else {
            eprintln!("跳过测试: 未设置讯飞凭证环境变量");
            return;
        };

        let client = XunfeiClient::new(app_id, api_key, api_secret);

        // 测试短音频（0.3秒）
        let audio = generate_silence_audio(0.3);
        assert!(!audio.is_valid(), "0.3秒音频应该被标记为无效");

        // 仍然尝试发送，看服务端如何处理
        let result = client.transcribe(&audio).await;
        println!("短音频测试结果: {result:?}");
    }

    #[tokio::test]
    async fn test_xunfei_long_audio() {
        let Some((app_id, api_key, api_secret)) = get_xunfei_credentials() else {
            eprintln!("跳过测试: 未设置讯飞凭证环境变量");
            return;
        };

        let client = XunfeiClient::new(app_id, api_key, api_secret);

        // 测试较长音频（5秒）
        let audio = generate_silence_audio(5.0);
        let result = client.transcribe(&audio).await;

        match result {
            Ok(r) => {
                println!("✅ 讯飞长音频测试成功");
                println!("   识别结果: {:?}", r.text);
            }
            Err(e) => {
                panic!("❌ 讯飞长音频测试失败: {e:?}");
            }
        }
    }
}

mod baidu {
    use super::*;
    use voice_core::asr_client::BaiduClient;

    fn get_baidu_credentials() -> Option<(String, String)> {
        let api_key = std::env::var("BAIDU_API_KEY").ok()?;
        let secret_key = std::env::var("BAIDU_SECRET_KEY").ok()?;
        Some((api_key, secret_key))
    }

    #[tokio::test]
    async fn test_baidu_connection() {
        let Some((api_key, secret_key)) = get_baidu_credentials() else {
            eprintln!("跳过测试: 未设置百度凭证环境变量");
            return;
        };

        let client = BaiduClient::new(api_key, secret_key);

        let audio = generate_silence_audio(1.0);
        let result = client.transcribe(&audio).await;

        match result {
            Ok(r) => {
                println!("✅ 百度连接成功");
                println!("   识别结果: {:?}", r.text);
            }
            Err(e) => {
                panic!("❌ 百度连接失败: {e:?}");
            }
        }
    }
}

mod openai {
    use super::*;
    use voice_core::asr_client::OpenAIWhisperClient;

    fn get_openai_credentials() -> Option<String> {
        let key = std::env::var("OPENAI_API_KEY").ok()?;
        if key.is_empty() {
            None
        } else {
            Some(key)
        }
    }

    #[tokio::test]
    async fn test_openai_connection() {
        let Some(api_key) = get_openai_credentials() else {
            eprintln!("跳过测试: 未设置 OpenAI 凭证环境变量");
            return;
        };

        let client = OpenAIWhisperClient::new(api_key);

        let audio = generate_silence_audio(1.0);
        let result = client.transcribe(&audio).await;

        match result {
            Ok(r) => {
                println!("✅ OpenAI 连接成功");
                println!("   识别结果: {:?}", r.text);
            }
            Err(e) => {
                panic!("❌ OpenAI 连接失败: {e:?}");
            }
        }
    }
}

/// 综合测试：测试所有已配置的 ASR 服务
#[tokio::test]
async fn test_all_configured_asr_services() {
    println!("\n========== ASR 服务综合测试 ==========\n");

    let mut tested = 0;
    let mut passed = 0;

    // 测试讯飞
    if let (Ok(app_id), Ok(api_key), Ok(api_secret)) = (
        std::env::var("XUNFEI_APP_ID"),
        std::env::var("XUNFEI_API_KEY"),
        std::env::var("XUNFEI_API_SECRET"),
    ) {
        tested += 1;
        let client = XunfeiClient::new(app_id, api_key, api_secret);
        let audio = generate_silence_audio(1.0);
        match client.transcribe(&audio).await {
            Ok(_) => {
                println!("✅ 讯飞: 连接正常");
                passed += 1;
            }
            Err(e) => println!("❌ 讯飞: {e:?}"),
        }
    } else {
        println!("⏭️  讯飞: 未配置");
    }

    // 测试百度
    if let (Ok(api_key), Ok(secret_key)) = (
        std::env::var("BAIDU_API_KEY"),
        std::env::var("BAIDU_SECRET_KEY"),
    ) {
        tested += 1;
        let client = voice_core::asr_client::BaiduClient::new(api_key, secret_key);
        let audio = generate_silence_audio(1.0);
        match client.transcribe(&audio).await {
            Ok(_) => {
                println!("✅ 百度: 连接正常");
                passed += 1;
            }
            Err(e) => println!("❌ 百度: {e:?}"),
        }
    } else {
        println!("⏭️  百度: 未配置");
    }

    // 测试 OpenAI
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        if !api_key.is_empty() {
            tested += 1;
            let client = voice_core::asr_client::OpenAIWhisperClient::new(api_key);
            let audio = generate_silence_audio(1.0);
            match client.transcribe(&audio).await {
                Ok(_) => {
                    println!("✅ OpenAI: 连接正常");
                    passed += 1;
                }
                Err(e) => println!("❌ OpenAI: {e:?}"),
            }
        } else {
            println!("⏭️  OpenAI: 未配置");
        }
    } else {
        println!("⏭️  OpenAI: 未配置");
    }

    println!("\n========== 测试结果 ==========");
    println!("测试: {passed}/{tested} 通过");

    if tested > 0 {
        assert_eq!(passed, tested, "部分 ASR 服务测试失败");
    }
}
