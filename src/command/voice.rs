use crate::config::YamlConfig;
use crate::constants::voice as vc;
use crate::{error, info};
use colored::Colorize;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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

/// 语音转文字命令入口
///
/// - action 为空：录音 → Whisper 转写 → 输出文字
/// - action 为 "download"：下载指定模型
/// - copy: 转写结果复制到剪贴板
/// - model_size: 指定模型大小 (tiny/base/small/medium/large)
pub fn handle_voice(action: &str, copy: bool, model_size: Option<&str>, _config: &YamlConfig) {
    let model = model_size.unwrap_or(vc::DEFAULT_MODEL);

    // 验证模型大小
    if !vc::MODEL_SIZES.contains(&model) {
        error!(
            "不支持的模型大小: {}，可选: {}",
            model,
            vc::MODEL_SIZES.join(", ")
        );
        return;
    }

    if action == vc::ACTION_DOWNLOAD {
        // 下载模型
        download_model(model);
        return;
    }

    if !action.is_empty() {
        error!("未知操作: {}，可用操作: download", action);
        crate::usage!("voice [-c] [-m <model>] 或 voice download [-m <model>]");
        return;
    }

    // 检查模型是否存在
    let model_path = get_model_path(model);
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

    // 检查模型文件完整性（文件大小是否达到期望最小值）
    let file_size_mb = std::fs::metadata(&model_path)
        .map(|m| m.len() / 1024 / 1024)
        .unwrap_or(0);
    let min_size = expected_min_size_mb(model);
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

    // 开始录音
    info!("🎙️  按 {} 开始录音...", "回车".green().bold());
    wait_for_enter();

    info!("🔴 录音中... 按 {} 结束录音", "回车".red().bold());

    let recording_path = get_recording_path();
    match record_audio(&recording_path) {
        Ok(()) => {
            info!("✅ 录音完成，开始转写...");
        }
        Err(e) => {
            error!("[handle_voice] 录音失败: {}", e);
            return;
        }
    }

    // Whisper 转写
    match transcribe(&model_path, &recording_path) {
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
            error!("[handle_voice] 转写失败: {}", e);
        }
    }

    // 清理临时录音文件
    let _ = std::fs::remove_file(&recording_path);
}

/// 获取模型文件路径: ~/.jdata/voice/model/ggml-<size>.bin
fn get_model_path(model_size: &str) -> PathBuf {
    let model_file = vc::MODEL_FILE_TEMPLATE.replace("{}", model_size);
    let voice_dir = YamlConfig::data_dir()
        .join(vc::VOICE_DIR)
        .join(vc::MODEL_DIR);
    let _ = std::fs::create_dir_all(&voice_dir);
    voice_dir.join(model_file)
}

/// 获取临时录音文件路径: ~/.jdata/voice/recording.wav
fn get_recording_path() -> PathBuf {
    let voice_dir = YamlConfig::data_dir().join(vc::VOICE_DIR);
    let _ = std::fs::create_dir_all(&voice_dir);
    voice_dir.join(vc::RECORDING_FILE)
}

/// 等待用户按回车
fn wait_for_enter() {
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
}

/// 录音：使用 cpal 捕获麦克风音频，保存为 WAV 文件
/// 使用设备默认配置录音，然后重采样到 16kHz 单声道（Whisper 要求）
/// 用户按回车结束录音
fn record_audio(output_path: &PathBuf) -> Result<(), String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "未找到麦克风设备，请检查音频输入设备".to_string())?;

    // 获取设备支持的默认输入配置
    let supported_config = device
        .default_input_config()
        .map_err(|e| format!("获取设备默认输入配置失败: {}", e))?;

    let device_sample_rate = supported_config.sample_rate();
    let device_channels = supported_config.channels();

    let config = cpal::StreamConfig {
        channels: device_channels,
        sample_rate: supported_config.sample_rate(),
        buffer_size: cpal::BufferSize::Default,
    };

    // 用于在录音线程和主线程之间共享数据
    let recording = Arc::new(AtomicBool::new(true));
    let recording_clone = recording.clone();

    // 收集原始 f32 音频采样数据（设备原始采样率和声道数）
    let raw_samples: Arc<std::sync::Mutex<Vec<f32>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let raw_samples_clone = raw_samples.clone();

    let err_flag: Arc<std::sync::Mutex<Option<String>>> = Arc::new(std::sync::Mutex::new(None));
    let err_flag_clone = err_flag.clone();

    // 创建音频输入流
    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if !recording_clone.load(Ordering::Relaxed) {
                    return;
                }
                let mut buf = raw_samples_clone.lock().unwrap();
                buf.extend_from_slice(data);
            },
            move |err| {
                let mut flag = err_flag_clone.lock().unwrap();
                *flag = Some(format!("录音流错误: {}", err));
            },
            None,
        )
        .map_err(|e| format!("创建录音流失败: {}", e))?;

    stream.play().map_err(|e| format!("启动录音失败: {}", e))?;

    // 等待用户按回车结束录音
    wait_for_enter();

    // 停止录音
    recording.store(false, Ordering::Relaxed);
    // 给录音流一点时间完成最后的数据收集
    std::thread::sleep(std::time::Duration::from_millis(100));
    drop(stream);

    // 检查是否有错误
    if let Some(err) = err_flag.lock().unwrap().take() {
        return Err(err);
    }

    let raw_data = raw_samples.lock().unwrap();
    if raw_data.is_empty() {
        return Err("未录到任何音频数据".to_string());
    }

    // 步骤 1: 多声道转单声道（取各声道均值）
    let mono_samples: Vec<f32> = if device_channels > 1 {
        raw_data
            .chunks(device_channels as usize)
            .map(|frame| frame.iter().sum::<f32>() / device_channels as f32)
            .collect()
    } else {
        raw_data.clone()
    };

    // 步骤 2: 重采样到 16kHz（如果设备采样率不是 16kHz）
    let target_rate = vc::SAMPLE_RATE;
    let resampled: Vec<f32> = if device_sample_rate != target_rate {
        resample(&mono_samples, device_sample_rate, target_rate)
    } else {
        mono_samples
    };

    // 步骤 3: 转换为 i16 并写入 WAV
    let i16_samples: Vec<i16> = resampled
        .iter()
        .map(|&s| {
            let clamped = s.clamp(-1.0, 1.0);
            (clamped * i16::MAX as f32) as i16
        })
        .collect();

    if i16_samples.is_empty() {
        return Err("重采样后无音频数据".to_string());
    }

    let duration_secs = i16_samples.len() as f64 / target_rate as f64;
    info!(
        "📊 录音时长: {:.1}s (设备: {}Hz {}ch → 重采样到 {}Hz 单声道)",
        duration_secs, device_sample_rate, device_channels, target_rate
    );

    let spec = hound::WavSpec {
        channels: vc::CHANNELS,
        sample_rate: target_rate,
        bits_per_sample: vc::BITS_PER_SAMPLE,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(output_path, spec)
        .map_err(|e| format!("创建 WAV 文件失败: {}", e))?;

    for &sample in i16_samples.iter() {
        writer
            .write_sample(sample)
            .map_err(|e| format!("写入音频数据失败: {}", e))?;
    }

    writer
        .finalize()
        .map_err(|e| format!("完成 WAV 文件写入失败: {}", e))?;

    Ok(())
}

/// 线性插值重采样
/// 将 source_rate 的音频数据重采样到 target_rate
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

/// 使用 Whisper 模型转写音频文件
fn transcribe(model_path: &PathBuf, audio_path: &PathBuf) -> Result<String, String> {
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    // 临时抑制 whisper.cpp C 库的 stderr 调试输出
    let _stderr_guard = suppress_stderr();

    // 加载模型
    let ctx = WhisperContext::new_with_params(
        model_path.to_str().unwrap_or(""),
        WhisperContextParameters::default(),
    )
    .map_err(|e| format!("加载 Whisper 模型失败: {}", e))?;

    let mut state = ctx
        .create_state()
        .map_err(|e| format!("创建 Whisper 状态失败: {}", e))?;

    // 读取 WAV 文件并转换为 f32 采样
    let reader =
        hound::WavReader::open(audio_path).map_err(|e| format!("读取 WAV 文件失败: {}", e))?;

    let samples: Vec<f32> = reader
        .into_samples::<i16>()
        .filter_map(|s| s.ok())
        .map(|s| s as f32 / i16::MAX as f32)
        .collect();

    if samples.is_empty() {
        return Err("音频文件为空".to_string());
    }

    // 配置 Whisper 转写参数
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

    // 设置语言为中文
    params.set_language(Some("zh"));
    // 不打印进度
    params.set_print_progress(false);
    // 不打印特殊 token
    params.set_print_special(false);
    // 不打印实时结果
    params.set_print_realtime(false);
    // 单段模式（适合短音频）
    params.set_single_segment(false);
    // 线程数
    params.set_n_threads(4);

    // 执行转写
    state
        .full(params, &samples)
        .map_err(|e| format!("Whisper 转写失败: {}", e))?;

    // 提取转写结果
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

/// 下载 Whisper 模型
fn download_model(model_size: &str) {
    let model_path = get_model_path(model_size);

    if model_path.exists() {
        let file_size = std::fs::metadata(&model_path).map(|m| m.len()).unwrap_or(0);
        let file_size_mb = file_size / 1024 / 1024;
        let min_size = expected_min_size_mb(model_size);

        if file_size_mb < min_size {
            // 文件存在但不完整（可能是之前下载中断）
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

    // 使用 curl 下载（避免引入额外的 HTTP 依赖）
    let status = std::process::Command::new("curl")
        .args([
            "-L",             // 跟随重定向
            "--progress-bar", // 进度条
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
    use std::io::Write;

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
/// 返回一个 guard，drop 时自动恢复 stderr
fn suppress_stderr() -> StderrGuard {
    use std::os::unix::io::AsRawFd;

    let stderr_fd = std::io::stderr().as_raw_fd();
    // 备份原始 stderr fd
    let saved_fd = unsafe { libc::dup(stderr_fd) };
    // 打开 /dev/null
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

/// stderr 重定向 guard，drop 时恢复原始 stderr
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
