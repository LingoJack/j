use crate::config::YamlConfig;
use crate::constants::voice as vc;
use crate::{error, info};
use colored::Colorize;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

// ========== 可复用基础函数 ==========

/// 获取模型期望的最小文件大小（MB），用于完整性校验
fn expected_min_size_mb(model_size: &str) -> u64 {
    match model_size {
        "tiny" => 70,
        "base" => 130,
        "small" => 450,
        "medium" => 1400,
        "large" => 2900,
        _ => 50,
    }
}

/// crossterm raw mode 的 RAII guard，确保异常时恢复终端
struct RawModeGuard;

impl RawModeGuard {
    fn enter() -> Result<Self, String> {
        crossterm::terminal::enable_raw_mode().map_err(|e| format!("启用 raw mode 失败: {}", e))?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

/// 启动录音流，返回 (stream, sample_rate, channels)
/// recording 控制录音开关，raw_samples 收集原始采样数据
fn start_recording_stream(
    recording: Arc<AtomicBool>,
    raw_samples: Arc<std::sync::Mutex<Vec<f32>>>,
) -> Result<(cpal::Stream, u32, u16), String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "未找到麦克风设备，请检查音频输入设备".to_string())?;

    let supported_config = device
        .default_input_config()
        .map_err(|e| format!("获取设备默认输入配置失败: {}", e))?;

    let sample_rate = supported_config.sample_rate();
    let channels = supported_config.channels();

    let config = cpal::StreamConfig {
        channels,
        sample_rate,
        buffer_size: cpal::BufferSize::Default,
    };

    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if !recording.load(Ordering::Relaxed) {
                    return;
                }
                let mut buf = raw_samples.lock().unwrap();
                buf.extend_from_slice(data);
            },
            move |err| {
                eprintln!("录音流错误: {}", err);
            },
            None,
        )
        .map_err(|e| format!("创建录音流失败: {}", e))?;

    stream.play().map_err(|e| format!("启动录音失败: {}", e))?;

    Ok((stream, sample_rate, channels))
}

/// 多声道转单声道 + 重采样到 16kHz
fn process_raw_audio(raw_data: &[f32], sample_rate: u32, channels: u16) -> Vec<f32> {
    // 多声道转单声道
    let mono: Vec<f32> = if channels > 1 {
        raw_data
            .chunks(channels as usize)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect()
    } else {
        raw_data.to_vec()
    };

    // 重采样到 16kHz
    let target_rate = vc::SAMPLE_RATE;
    if sample_rate != target_rate {
        resample(&mono, sample_rate, target_rate)
    } else {
        mono
    }
}

/// 直接从 f32 samples 转写（不经过 WAV 文件）
fn transcribe_from_samples(model_path: &PathBuf, samples: &[f32]) -> Result<String, String> {
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    if samples.is_empty() {
        return Err("音频数据为空".to_string());
    }

    let _stderr_guard = suppress_stderr();

    let ctx = WhisperContext::new_with_params(
        model_path.to_str().unwrap_or(""),
        WhisperContextParameters::default(),
    )
    .map_err(|e| format!("加载 Whisper 模型失败: {}", e))?;

    let mut state = ctx
        .create_state()
        .map_err(|e| format!("创建 Whisper 状态失败: {}", e))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("zh"));
    params.set_print_progress(false);
    params.set_print_special(false);
    params.set_print_realtime(false);
    params.set_single_segment(false);
    params.set_n_threads(4);

    state
        .full(params, samples)
        .map_err(|e| format!("Whisper 转写失败: {}", e))?;

    let num_segments = state.full_n_segments();
    let mut result = String::new();
    for i in 0..num_segments {
        if let Some(segment) = state.get_segment(i) {
            if let Ok(text) = segment.to_str_lossy() {
                result.push_str(&text);
            }
        }
    }

    Ok(result)
}

/// 自动检测最佳可用模型，按 large > medium > small > base > tiny 优先级
fn detect_best_model() -> Option<&'static str> {
    for &size in vc::MODEL_PRIORITY {
        let path = get_model_path(size);
        if path.exists() {
            let file_size_mb = std::fs::metadata(&path)
                .map(|m| m.len() / 1024 / 1024)
                .unwrap_or(0);
            if file_size_mb >= expected_min_size_mb(size) {
                return Some(size);
            }
        }
    }
    None
}

/// 使用 crossterm raw mode 等待按键停止录音
/// 返回 true 表示用户按了停止键，false 表示录音标志已被外部清除
fn wait_for_stop_key(recording: &AtomicBool) -> bool {
    use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

    loop {
        if !recording.load(Ordering::Relaxed) {
            return false;
        }
        if event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
            if let Ok(Event::Key(KeyEvent {
                code, modifiers, ..
            })) = event::read()
            {
                match code {
                    KeyCode::Enter => return true,
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        return true;
                    }
                    _ => {}
                }
            }
        }
    }
}

/// 使用 crossterm raw mode 等待 Ctrl+V 停止录音（交互模式专用）
fn wait_for_ctrl_v_stop(recording: &AtomicBool) -> bool {
    use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

    loop {
        if !recording.load(Ordering::Relaxed) {
            return false;
        }
        if event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
            if let Ok(Event::Key(KeyEvent {
                code, modifiers, ..
            })) = event::read()
            {
                match code {
                    KeyCode::Char('v') if modifiers.contains(KeyModifiers::CONTROL) => {
                        return true;
                    }
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        return true;
                    }
                    _ => {}
                }
            }
        }
    }
}

// ========== 流式转写 ==========

/// 录音 + 流式转写：边录边显示
/// 返回最终完整转写文本
fn record_and_transcribe_streaming(model_path: &PathBuf) -> Result<String, String> {
    let recording = Arc::new(AtomicBool::new(true));
    let raw_samples: Arc<std::sync::Mutex<Vec<f32>>> = Arc::new(std::sync::Mutex::new(Vec::new()));

    let (stream, sample_rate, channels) =
        start_recording_stream(recording.clone(), raw_samples.clone())?;

    // 流式转写线程
    let streaming_recording = recording.clone();
    let streaming_samples = raw_samples.clone();
    let streaming_model = model_path.clone();
    let streaming_sr = sample_rate;
    let streaming_ch = channels;
    let displayed_len = Arc::new(std::sync::Mutex::new(0usize));
    let displayed_len_clone = displayed_len.clone();

    let transcribe_handle = std::thread::spawn(move || {
        let interval = std::time::Duration::from_secs(vc::STREAMING_INTERVAL_SECS);
        let min_samples = (vc::MIN_AUDIO_SECS as usize) * (streaming_sr as usize);

        while streaming_recording.load(Ordering::Relaxed) {
            std::thread::sleep(interval);

            if !streaming_recording.load(Ordering::Relaxed) {
                break;
            }

            let raw_data = streaming_samples.lock().unwrap().clone();
            // 需要足够的原始采样数据才尝试转写
            if raw_data.len() < min_samples * (streaming_ch as usize) {
                continue;
            }

            let processed = process_raw_audio(&raw_data, streaming_sr, streaming_ch);
            if processed.is_empty() {
                continue;
            }

            if let Ok(text) = transcribe_from_samples(&streaming_model, &processed) {
                let text = text.trim().to_string();
                let mut prev_len = displayed_len_clone.lock().unwrap();
                if text.len() > *prev_len {
                    let new_part = &text[*prev_len..];
                    print!("{}", new_part);
                    let _ = std::io::stdout().flush();
                    *prev_len = text.len();
                }
            }
        }
    });

    // 进入 raw mode 等待用户按键停止
    let _raw_guard = RawModeGuard::enter()?;
    wait_for_stop_key(&recording);
    drop(_raw_guard);

    // 停止录音
    recording.store(false, Ordering::Relaxed);
    std::thread::sleep(std::time::Duration::from_millis(100));
    drop(stream);

    let _ = transcribe_handle.join();

    // 最终完整转写确保精度
    let raw_data = raw_samples.lock().unwrap();
    if raw_data.is_empty() {
        return Err("未录到任何音频数据".to_string());
    }

    let processed = process_raw_audio(&raw_data, sample_rate, channels);
    let duration_secs = processed.len() as f64 / vc::SAMPLE_RATE as f64;

    // 换行（之前流式输出可能没换行）
    println!();
    info!(
        "📊 录音时长: {:.1}s (设备: {}Hz {}ch → 16kHz 单声道)",
        duration_secs, sample_rate, channels
    );

    if processed.is_empty() || duration_secs < vc::MIN_AUDIO_SECS as f64 {
        return Err("录音时间过短".to_string());
    }

    // 清除之前的流式输出，用最终结果替代
    let prev_len = *displayed_len.lock().unwrap();
    let final_text = transcribe_from_samples(model_path, &processed)?;
    let final_text = final_text.trim().to_string();

    // 如果最终结果与流式结果不同，重新输出
    if final_text.len() != prev_len {
        // 已经换行了，直接输出完整最终结果
    }

    Ok(final_text)
}

// ========== CLI 入口 ==========

/// 语音转文字命令入口
///
/// - action 为空：录音 → Whisper 流式转写 → 输出文字
/// - action 为 "download"：下载指定模型
/// - copy: 转写结果复制到剪贴板
/// - model_size: 指定模型大小 (tiny/base/small/medium/large)，为 None 时自动检测
pub fn handle_voice(action: &str, copy: bool, model_size: Option<&str>, _config: &YamlConfig) {
    // 如果用户指定了模型，使用指定的；否则自动检测，再降级到默认
    let model = if let Some(m) = model_size {
        m.to_string()
    } else if let Some(best) = detect_best_model() {
        info!("🔍 自动检测到模型: {}", best.cyan().bold());
        best.to_string()
    } else {
        vc::DEFAULT_MODEL.to_string()
    };

    // 验证模型大小
    if !vc::MODEL_SIZES.contains(&model.as_str()) {
        error!(
            "不支持的模型大小: {}，可选: {}",
            model,
            vc::MODEL_SIZES.join(", ")
        );
        return;
    }

    if action == vc::ACTION_DOWNLOAD {
        download_model(&model);
        return;
    }

    if !action.is_empty() {
        error!("未知操作: {}，可用操作: download", action);
        crate::usage!("voice [-c] [-m <model>] 或 voice download [-m <model>]");
        return;
    }

    // 检查模型是否存在
    let model_path = get_model_path(&model);
    if !model_path.exists() {
        error!("模型文件不存在: {}", model_path.display());
        info!(
            "💡 请先下载模型: {} 或 {}",
            format!("j voice download -m {}", model).cyan(),
            format!("j voice download").cyan()
        );
        info!(
            "💡 也可以手动下载模型放到: {}",
            model_path.display().to_string().cyan()
        );
        return;
    }

    // 检查模型文件完整性
    let file_size_mb = std::fs::metadata(&model_path)
        .map(|m| m.len() / 1024 / 1024)
        .unwrap_or(0);
    let min_size = expected_min_size_mb(&model);
    if file_size_mb < min_size {
        error!(
            "模型文件不完整: {} ({} MB，期望至少 {} MB)",
            model_path.display(),
            file_size_mb,
            min_size
        );
        info!(
            "💡 请删除后重新下载: {} && {}",
            format!("rm {}", model_path.display()).cyan(),
            format!("j voice download -m {}", model).cyan()
        );
        return;
    }

    info!(
        "🎙️  按 {} 开始录音，录音中按 {} 或 {} 结束",
        "回车".green().bold(),
        "回车".red().bold(),
        "Ctrl+C".red().bold()
    );

    // 等待用户按回车开始（使用 crossterm raw mode 避免与交互模式冲突）
    {
        let _raw_guard = match RawModeGuard::enter() {
            Ok(g) => g,
            Err(e) => {
                error!("[handle_voice] {}", e);
                return;
            }
        };
        use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
        loop {
            if event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
                if let Ok(Event::Key(KeyEvent {
                    code, modifiers, ..
                })) = event::read()
                {
                    match code {
                        KeyCode::Enter => break,
                        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                            return;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    println!();
    info!(
        "🔴 录音中... 按 {} 或 {} 结束录音",
        "回车".red().bold(),
        "Ctrl+C".red().bold()
    );

    match record_and_transcribe_streaming(&model_path) {
        Ok(text) => {
            let text = text.trim().to_string();
            if text.is_empty() {
                info!("⚠️  未识别到语音内容");
            } else {
                println!();
                info!("📝 转写结果:");
                println!("{}", text);

                if copy {
                    copy_to_clipboard(&text);
                }
            }
        }
        Err(e) => {
            error!("[handle_voice] {}", e);
        }
    }
}

// ========== 交互模式录音入口 ==========

/// 交互模式下的语音录音入口（由 Ctrl+V 或 voice 命令触发）
/// 返回转写文本（可能为空字符串）
pub fn do_voice_record_for_interactive() -> String {
    let model = if let Some(best) = detect_best_model() {
        info!("🔍 自动检测到模型: {}", best.cyan().bold());
        best.to_string()
    } else {
        vc::DEFAULT_MODEL.to_string()
    };

    let model_path = get_model_path(&model);
    if !model_path.exists() {
        error!("模型文件不存在: {}", model_path.display());
        info!("💡 请先下载模型: {}", format!("j voice download").cyan());
        return String::new();
    }

    let file_size_mb = std::fs::metadata(&model_path)
        .map(|m| m.len() / 1024 / 1024)
        .unwrap_or(0);
    if file_size_mb < expected_min_size_mb(&model) {
        error!("模型文件不完整，请重新下载");
        return String::new();
    }

    info!(
        "🔴 录音中... 按 {} 或 {} 结束",
        "Ctrl+V".red().bold(),
        "Ctrl+C".red().bold()
    );

    let recording = Arc::new(AtomicBool::new(true));
    let raw_samples: Arc<std::sync::Mutex<Vec<f32>>> = Arc::new(std::sync::Mutex::new(Vec::new()));

    let (stream, sample_rate, channels) =
        match start_recording_stream(recording.clone(), raw_samples.clone()) {
            Ok(r) => r,
            Err(e) => {
                error!("[voice] {}", e);
                return String::new();
            }
        };

    // 流式转写线程
    let streaming_recording = recording.clone();
    let streaming_samples = raw_samples.clone();
    let streaming_model = model_path.clone();
    let streaming_sr = sample_rate;
    let streaming_ch = channels;
    let displayed_len = Arc::new(std::sync::Mutex::new(0usize));
    let displayed_len_clone = displayed_len.clone();

    let transcribe_handle = std::thread::spawn(move || {
        let interval = std::time::Duration::from_secs(vc::STREAMING_INTERVAL_SECS);
        let min_samples = (vc::MIN_AUDIO_SECS as usize) * (streaming_sr as usize);

        while streaming_recording.load(Ordering::Relaxed) {
            std::thread::sleep(interval);
            if !streaming_recording.load(Ordering::Relaxed) {
                break;
            }

            let raw_data = streaming_samples.lock().unwrap().clone();
            if raw_data.len() < min_samples * (streaming_ch as usize) {
                continue;
            }

            let processed = process_raw_audio(&raw_data, streaming_sr, streaming_ch);
            if processed.is_empty() {
                continue;
            }

            if let Ok(text) = transcribe_from_samples(&streaming_model, &processed) {
                let text = text.trim().to_string();
                let mut prev_len = displayed_len_clone.lock().unwrap();
                if text.len() > *prev_len {
                    let new_part = &text[*prev_len..];
                    // 在 raw mode 下需要用 \r\n
                    print!("{}", new_part);
                    let _ = std::io::stdout().flush();
                    *prev_len = text.len();
                }
            }
        }
    });

    // 进入 raw mode 等待 Ctrl+V 停止
    let raw_result = RawModeGuard::enter();
    if let Err(e) = &raw_result {
        error!("[voice] {}", e);
        recording.store(false, Ordering::Relaxed);
        let _ = transcribe_handle.join();
        drop(stream);
        return String::new();
    }
    let _raw_guard = raw_result.unwrap();
    wait_for_ctrl_v_stop(&recording);
    drop(_raw_guard);

    // 停止录音
    recording.store(false, Ordering::Relaxed);
    std::thread::sleep(std::time::Duration::from_millis(100));
    drop(stream);

    let _ = transcribe_handle.join();

    // 最终完整转写
    let raw_data = raw_samples.lock().unwrap();
    if raw_data.is_empty() {
        println!();
        info!("⚠️  未录到音频数据");
        return String::new();
    }

    let processed = process_raw_audio(&raw_data, sample_rate, channels);
    let duration_secs = processed.len() as f64 / vc::SAMPLE_RATE as f64;

    println!();
    info!("📊 录音时长: {:.1}s", duration_secs);

    if processed.is_empty() || duration_secs < vc::MIN_AUDIO_SECS as f64 {
        info!("⚠️  录音时间过短");
        return String::new();
    }

    info!("✅ 转写中...");
    match transcribe_from_samples(&model_path, &processed) {
        Ok(text) => {
            let text = text.trim().to_string();
            if text.is_empty() {
                info!("⚠️  未识别到语音内容");
            } else {
                info!("📝 {}", &text);
            }
            text
        }
        Err(e) => {
            error!("[voice] 转写失败: {}", e);
            String::new()
        }
    }
}

// ========== 辅助函数 ==========

/// 获取模型文件路径: ~/.jdata/voice/model/ggml-<size>.bin
fn get_model_path(model_size: &str) -> PathBuf {
    let model_file = vc::MODEL_FILE_TEMPLATE.replace("{}", model_size);
    let voice_dir = YamlConfig::data_dir()
        .join(vc::VOICE_DIR)
        .join(vc::MODEL_DIR);
    let _ = std::fs::create_dir_all(&voice_dir);
    voice_dir.join(model_file)
}

/// 线性插值重采样
fn resample(samples: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
    if samples.is_empty() || source_rate == target_rate {
        return samples.to_vec();
    }

    let ratio = source_rate as f64 / target_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_idx = i as f64 * ratio;
        let idx_floor = src_idx as usize;
        let frac = (src_idx - idx_floor as f64) as f32;

        let sample = if idx_floor + 1 < samples.len() {
            samples[idx_floor] * (1.0 - frac) + samples[idx_floor + 1] * frac
        } else if idx_floor < samples.len() {
            samples[idx_floor]
        } else {
            0.0
        };

        output.push(sample);
    }

    output
}

/// 下载 Whisper 模型
fn download_model(model_size: &str) {
    let model_path = get_model_path(model_size);

    if model_path.exists() {
        let file_size = std::fs::metadata(&model_path).map(|m| m.len()).unwrap_or(0);
        let file_size_mb = file_size / 1024 / 1024;
        let min_size = expected_min_size_mb(model_size);

        if file_size_mb < min_size {
            info!(
                "⚠️  模型文件不完整: {} ({} MB，期望至少 {} MB)",
                model_path.display(),
                file_size_mb,
                min_size
            );
            info!("🔄 删除不完整文件，重新下载...");
            let _ = std::fs::remove_file(&model_path);
        } else {
            info!(
                "✅ 模型已存在: {} ({:.1} MB)",
                model_path.display(),
                file_size as f64 / 1024.0 / 1024.0
            );
            info!("💡 如需重新下载，请先删除模型文件");
            return;
        }
    }

    let url = vc::MODEL_URL_TEMPLATE.replace("{}", model_size);

    info!("📥 下载 Whisper {} 模型...", model_size.cyan().bold());
    info!("   URL: {}", url.dimmed());
    info!("   保存到: {}", model_path.display().to_string().dimmed());
    println!();

    let status = std::process::Command::new("curl")
        .args([
            "-L",
            "--progress-bar",
            "-o",
            model_path.to_str().unwrap_or(""),
            &url,
        ])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();

    match status {
        Ok(s) if s.success() => {
            let file_size = std::fs::metadata(&model_path).map(|m| m.len()).unwrap_or(0);
            let file_size_mb = file_size / 1024 / 1024;
            let min_size = expected_min_size_mb(model_size);
            if file_size_mb < min_size {
                error!(
                    "下载的文件不完整 ({} MB，期望至少 {} MB)",
                    file_size_mb, min_size
                );
                error!(
                    "请检查网络连接，或手动下载模型文件到: {}",
                    model_path.display()
                );
                error!(
                    "手动下载链接: {}",
                    vc::MODEL_URL_TEMPLATE.replace("{}", model_size)
                );
                let _ = std::fs::remove_file(&model_path);
                return;
            }
            println!();
            info!(
                "✅ 模型下载完成: {} ({:.1} MB)",
                model_size.green().bold(),
                file_size as f64 / 1024.0 / 1024.0
            );
        }
        Ok(_) => {
            error!("模型下载失败，请检查网络连接");
            let _ = std::fs::remove_file(&model_path);
        }
        Err(e) => {
            error!(
                "[download_model] 执行 curl 失败: {}，请确保系统安装了 curl",
                e
            );
        }
    }
}

/// 复制文字到系统剪贴板 (macOS: pbcopy)
fn copy_to_clipboard(text: &str) {
    let mut child = match std::process::Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            error!("[copy_to_clipboard] 无法调用 pbcopy: {}", e);
            return;
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes());
    }

    match child.wait() {
        Ok(_) => info!("📋 已复制到剪贴板"),
        Err(e) => error!("[copy_to_clipboard] pbcopy 执行失败: {}", e),
    }
}

/// 临时抑制 stderr 输出（用于屏蔽 whisper.cpp C 库的调试日志）
fn suppress_stderr() -> StderrGuard {
    use std::os::unix::io::AsRawFd;

    let stderr_fd = std::io::stderr().as_raw_fd();
    let saved_fd = unsafe { libc::dup(stderr_fd) };
    let devnull = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .ok();
    if let Some(ref devnull_file) = devnull {
        unsafe {
            libc::dup2(devnull_file.as_raw_fd(), stderr_fd);
        }
    }

    StderrGuard {
        saved_fd,
        stderr_fd,
        _devnull: devnull,
    }
}

struct StderrGuard {
    saved_fd: i32,
    stderr_fd: i32,
    _devnull: Option<std::fs::File>,
}

impl Drop for StderrGuard {
    fn drop(&mut self) {
        if self.saved_fd >= 0 {
            unsafe {
                libc::dup2(self.saved_fd, self.stderr_fd);
                libc::close(self.saved_fd);
            }
        }
    }
}
