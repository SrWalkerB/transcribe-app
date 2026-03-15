use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use chrono::Local;
use serde::Serialize;
use tauri::path::BaseDirectory;
use tauri::{Emitter, Manager};

/// Em dev, tenta usar o Python do venv do backend (onde faster_whisper está instalado).
fn backend_venv_python(script_path: &Path) -> Option<String> {
    let mut dir = script_path.parent()?;
    for _ in 0..8 {
        let venv_python = dir.join("backend").join(".venv").join("bin").join("python3");
        if venv_python.is_file() {
            return venv_python.to_str().map(String::from);
        }
        dir = dir.parent()?;
    }
    None
}

#[derive(Serialize)]
struct DependencyStatus {
    ffmpeg: bool,
    python: bool,
    faster_whisper: bool,
}

/// Shared state: holds the running Python child process so cancel can kill it immediately.
struct TranscribeProcess(Arc<Mutex<Option<Child>>>);

#[tauri::command]
fn check_dependencies(app: tauri::AppHandle) -> DependencyStatus {
    let ffmpeg = Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    let python_bin = app
        .path()
        .resolve("resources/transcribe.py", BaseDirectory::Resource)
        .ok()
        .and_then(|p| backend_venv_python(&p))
        .unwrap_or_else(|| "python3".to_string());

    let python = Command::new(&python_bin)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    let faster_whisper = if python {
        Command::new(&python_bin)
            .arg("-c")
            .arg("import faster_whisper")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        false
    };

    DependencyStatus {
        ffmpeg,
        python,
        faster_whisper,
    }
}

#[tauri::command]
fn install_dependencies(app: tauri::AppHandle) -> Result<String, String> {
    let python_bin = app
        .path()
        .resolve("resources/transcribe.py", BaseDirectory::Resource)
        .ok()
        .and_then(|p| backend_venv_python(&p))
        .unwrap_or_else(|| "python3".to_string());

    let python_ok = Command::new(&python_bin)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !python_ok {
        return Err(
            "Python 3 não encontrado. Instale o Python 3 primeiro: https://www.python.org/downloads/"
                .to_string(),
        );
    }

    let output = Command::new(&python_bin)
        .arg("-m")
        .arg("pip")
        .arg("install")
        .arg("faster-whisper")
        .output()
        .map_err(|e| format!("Falha ao executar pip: {}", e))?;

    if output.status.success() {
        Ok("faster-whisper instalado com sucesso!".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "Falha ao instalar faster-whisper.\n\nTente manualmente: pip install faster-whisper\n\nDetalhe: {}",
            stderr.lines().take(5).collect::<Vec<_>>().join("\n")
        ))
    }
}

#[tauri::command]
fn get_video_duration(path: String) -> Result<f64, String> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("quiet")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("csv=p=0")
        .arg(&path)
        .output()
        .map_err(|_| "ffprobe não encontrado".to_string())?;

    if !output.status.success() {
        return Err("Falha ao obter duração do vídeo".to_string());
    }

    let dur_str = String::from_utf8_lossy(&output.stdout);
    dur_str
        .trim()
        .parse::<f64>()
        .map_err(|_| "Duração inválida".to_string())
}

#[tauri::command]
async fn transcribe_video(
    app: tauri::AppHandle,
    path: String,
    model: String,
    threads: u32,
    process_state: tauri::State<'_, TranscribeProcess>,
) -> Result<String, String> {
    {
        let mut guard = process_state.0.lock().unwrap();
        *guard = None;
    }

    let ps = process_state.0.clone();

    tokio::task::spawn_blocking(move || {
        transcribe_video_blocking(&app, &path, &model, threads, &ps)
    })
    .await
    .map_err(|e| format!("Erro interno: {}", e))?
}

#[tauri::command]
fn cancel_transcription(process_state: tauri::State<'_, TranscribeProcess>) {
    let mut guard = process_state.0.lock().unwrap();
    if let Some(ref mut child) = *guard {
        let _ = child.kill();
    }
}

#[tauri::command]
fn get_cpu_count() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
}

fn transcribe_video_blocking(
    app: &tauri::AppHandle,
    path: &str,
    model: &str,
    threads: u32,
    process_state: &Mutex<Option<Child>>,
) -> Result<String, String> {
    let video_path = Path::new(path);
    if !video_path.exists() {
        return Err(format!("Arquivo não encontrado: {}", path));
    }

    let video_filename = video_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("video");
    let timestamp = Local::now().timestamp();
    let mp3_name = format!("{}_{}.mp3", timestamp, video_filename);

    let temp_dir = std::env::temp_dir().join("transcribe-app");
    let output_text_dir = temp_dir.join("output-text");
    fs::create_dir_all(&output_text_dir)
        .map_err(|e| format!("Falha ao criar diretório temporário: {}", e))?;

    let mp3_path = temp_dir.join(&mp3_name);

    // Passo 1: ffmpeg vídeo -> MP3
    let _ = app.emit("transcribe-step", "audio");
    let ffmpeg_out = Command::new("ffmpeg")
        .arg("-i")
        .arg(path)
        .arg("-b:a")
        .arg("128k")
        .arg("-y")
        .arg(&mp3_path)
        .output()
        .map_err(|e| format!("Falha ao executar ffmpeg (está instalado?): {}", e))?;

    if !ffmpeg_out.status.success() {
        let stderr = String::from_utf8_lossy(&ffmpeg_out.stderr);
        return Err(format!(
            "Não foi possível converter o vídeo em áudio. Verifique se o ffmpeg está instalado no sistema.\n\nDetalhe: {}",
            stderr.lines().take(3).collect::<Vec<_>>().join(" ")
        ));
    }

    let mp3_path_abs = mp3_path
        .canonicalize()
        .map_err(|e| format!("Path do MP3 inválido: {}", e))?;
    let mp3_path_str = mp3_path_abs
        .to_str()
        .ok_or("Path do MP3 inválido (encoding)")?;

    let script_path = app
        .path()
        .resolve("resources/transcribe.py", BaseDirectory::Resource)
        .map_err(|e| format!("Script transcribe.py não encontrado no app: {:?}", e))?;
    let script_path_str = script_path
        .to_str()
        .ok_or("Path do script inválido (encoding)")?;

    // Passo 2: transcrição com Python (streaming)
    let _ = app.emit("transcribe-step", "text");
    let python_bin = backend_venv_python(&script_path).unwrap_or_else(|| "python3".to_string());

    let mut child = Command::new(&python_bin)
        .arg(script_path_str)
        .arg(&mp3_name)
        .arg(mp3_path_str)
        .arg("--model")
        .arg(model)
        .arg("--threads")
        .arg(threads.to_string())
        .current_dir(&temp_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Falha ao executar Python ({} está no PATH?): {}", python_bin, e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or("Falha ao capturar stdout do Python")?;
    let stderr_pipe = child.stderr.take();

    // Store child in shared state so cancel_transcription can kill it
    {
        let mut guard = process_state.lock().unwrap();
        *guard = Some(child);
    }

    let reader = BufReader::new(stdout);
    let mut duration: f64 = 0.0;
    let mut full_text = String::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break, // pipe closed (process killed)
        };

        if let Some(dur) = line.strip_prefix("__DURATION__:") {
            duration = dur.parse::<f64>().unwrap_or(0.0);
        } else if let Some(lang) = line.strip_prefix("__LANG__:") {
            let _ = app.emit("transcribe-lang", lang.to_string());
        } else if let Some(seg) = line.strip_prefix("__SEG__:") {
            if let Some((end_str, text)) = seg.split_once('|') {
                let end: f64 = end_str.parse().unwrap_or(0.0);
                let progress = if duration > 0.0 {
                    ((end / duration) * 100.0).min(100.0)
                } else {
                    0.0
                };

                if !full_text.is_empty() {
                    full_text.push(' ');
                }
                full_text.push_str(text);

                let _ = app.emit(
                    "transcribe-progress",
                    serde_json::json!({
                        "progress": progress,
                        "text": text,
                        "full_text": full_text,
                    }),
                );
            }
        } else if line == "__DONE__" {
            break;
        }
    }

    // Wait for process to finish and check result
    let mut guard = process_state.lock().unwrap();
    let cancelled = if let Some(ref mut child) = *guard {
        let status = child.wait();
        match status {
            Ok(s) => !s.success(),
            Err(_) => true,
        }
    } else {
        // child was already taken/killed
        true
    };
    // Clear the process from state
    *guard = None;
    drop(guard);

    // If process was killed (cancelled) or ended abnormally
    if cancelled {
        // Check if it was a user cancel (full_text exists but process was killed)
        if !full_text.is_empty() {
            return Err(format!("__CANCELLED__{}", full_text));
        }

        // Check stderr for specific errors
        let stderr_output = stderr_pipe
            .map(|s| {
                let mut buf = String::new();
                let mut reader = BufReader::new(s);
                let _ = reader.read_line(&mut buf);
                buf
            })
            .unwrap_or_default();

        if stderr_output.contains("faster_whisper") || stderr_output.contains("ModuleNotFoundError")
        {
            return Err(
                "Para transcrever, é necessário ter o módulo faster-whisper instalado.\n\n\
                Volte para a tela inicial e clique em \"Verificar dependências\" para instalar automaticamente."
                    .to_string(),
            );
        } else if stderr_output.contains("python3")
            && (stderr_output.contains("not found") || stderr_output.contains("No such file"))
        {
            return Err(
                "Python não foi encontrado no sistema. Instale Python 3 e o módulo faster-whisper."
                    .to_string(),
            );
        }

        return Err(format!(
            "__CANCELLED__{}",
            full_text
        ));
    }

    // Read from the file (Python writes there too) as the authoritative result
    let txt_path = output_text_dir.join(format!("{}.txt", mp3_name));
    let content = fs::read_to_string(&txt_path).map_err(|e| {
        format!(
            "Falha ao ler resultado da transcrição ({}): {}",
            txt_path.display(),
            e
        )
    })?;

    Ok(content)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(TranscribeProcess(Arc::new(Mutex::new(None))))
        .invoke_handler(tauri::generate_handler![
            transcribe_video,
            check_dependencies,
            install_dependencies,
            get_cpu_count,
            get_video_duration,
            cancel_transcription
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
