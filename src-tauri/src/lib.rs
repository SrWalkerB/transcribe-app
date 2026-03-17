use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use chrono::Local;
use serde::Serialize;
use tauri::path::BaseDirectory;
use tauri::{Emitter, Manager};

/// Check if we're running inside an AppImage.
fn is_appimage() -> bool {
    std::env::var("APPIMAGE").is_ok()
}

/// Local folder used to store downloaded FFmpeg binaries.
fn local_ffmpeg_bin_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|base| base.join("transcribe-app").join("ffmpeg").join("bin"))
}

fn installed_command_path(name: &str) -> Option<String> {
    let dir = local_ffmpeg_bin_dir()?;
    let filename = if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };
    let path = dir.join(filename);
    if path.is_file() {
        Some(path.to_string_lossy().to_string())
    } else {
        None
    }
}

/// Resolve a command by searching common installation paths (Homebrew, system, etc.)
/// This is needed because GUI apps on macOS may not inherit the full shell PATH,
/// and AppImage bundles its own binaries that may conflict with system ones.
fn resolve_command(name: &str) -> String {
    resolve_command_any(&[name])
}

fn resolve_command_any(names: &[&str]) -> String {
    for name in names {
        if let Some(installed) = installed_command_path(name) {
            return installed;
        }
    }

    let extra_paths = [
        "/opt/homebrew/bin",      // macOS Apple Silicon (Homebrew)
        "/usr/local/bin",         // macOS Intel (Homebrew) / Linux
        "/usr/bin",               // System default
        "/snap/bin",              // Linux snap packages
    ];

    // Inside AppImage, skip paths that are inside the AppImage mount
    let appdir = std::env::var("APPDIR").unwrap_or_default();

    for name in names {
        for dir in &extra_paths {
            // If inside AppImage, make sure we're finding the REAL system binary
            if !appdir.is_empty() && dir.starts_with("/usr") {
                // Check the real system path, not the AppImage overlay
                let real_path = format!("{}/{}", dir, name);
                // Verify it's not inside the AppImage mount
                if let Ok(canonical) = std::fs::canonicalize(&real_path) {
                    let canon_str = canonical.to_string_lossy();
                    if canon_str.contains(".mount_") || canon_str.starts_with(&appdir) {
                        continue; // skip AppImage's bundled binary
                    }
                    return canonical.to_string_lossy().to_string();
                }
                continue;
            }

            let full = format!("{}/{}", dir, name);
            if Path::new(&full).is_file() {
                return full;
            }
        }
    }

    names.first().unwrap_or(&"").to_string() // fallback to PATH lookup
}

/// Create a Command that clears AppImage Python environment variables.
/// AppImage sets PYTHONHOME/PYTHONPATH to its internal paths, which breaks
/// the system Python's ability to find/install packages.
fn python_command(python_bin: &str) -> Command {
    let mut cmd = Command::new(python_bin);
    if is_appimage() {
        cmd.env_remove("PYTHONHOME");
        cmd.env_remove("PYTHONPATH");
    }
    cmd
}

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

/// Resolve Python: first try venv, then common paths.
fn resolve_python(script_path: Option<&Path>) -> String {
    if let Some(sp) = script_path {
        if let Some(venv) = backend_venv_python(sp) {
            return venv;
        }
    }
    resolve_command_any(&["python3", "python"])
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
async fn check_dependencies(app: tauri::AppHandle) -> DependencyStatus {
    let script_path = app
        .path()
        .resolve("resources/transcribe.py", BaseDirectory::Resource)
        .ok();

    tokio::task::spawn_blocking(move || {
        let ffmpeg_bin = resolve_command("ffmpeg");
        let ffmpeg = Command::new(&ffmpeg_bin)
            .arg("-version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        let python_bin = resolve_python(script_path.as_deref());

        let python = python_command(&python_bin)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        let faster_whisper = if python {
            python_command(&python_bin)
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
    })
    .await
    .unwrap_or(DependencyStatus {
        ffmpeg: false,
        python: false,
        faster_whisper: false,
    })
}

#[tauri::command]
async fn install_dependencies(app: tauri::AppHandle) -> Result<String, String> {
    let script_path = app
        .path()
        .resolve("resources/transcribe.py", BaseDirectory::Resource)
        .ok();
    let python_bin = resolve_python(script_path.as_deref());

    tokio::task::spawn_blocking(move || install_dependencies_blocking(&python_bin))
        .await
        .map_err(|e| format!("Erro interno: {}", e))?
}

fn install_dependencies_blocking(python_bin: &str) -> Result<String, String> {
    let python_ok = python_command(python_bin)
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

    // Strategy 1: pip install --user (works on externally-managed environments)
    let output = python_command(python_bin)
        .arg("-m")
        .arg("pip")
        .arg("install")
        .arg("--user")
        .arg("faster-whisper")
        .output()
        .map_err(|e| format!("Falha ao executar pip: {}", e))?;

    if output.status.success() {
        return Ok("faster-whisper instalado com sucesso!".to_string());
    }

    // Strategy 2: pip install --break-system-packages (fallback for strict environments)
    let output2 = python_command(python_bin)
        .arg("-m")
        .arg("pip")
        .arg("install")
        .arg("--break-system-packages")
        .arg("faster-whisper")
        .output()
        .map_err(|e| format!("Falha ao executar pip: {}", e))?;

    if output2.status.success() {
        return Ok("faster-whisper instalado com sucesso!".to_string());
    }

    // Strategy 3: pipx install (if pipx is available)
    let pipx_result = Command::new("pipx")
        .arg("install")
        .arg("faster-whisper")
        .output();

    if let Ok(pipx_out) = pipx_result {
        if pipx_out.status.success() {
            return Ok("faster-whisper instalado com sucesso via pipx!".to_string());
        }
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "Falha ao instalar faster-whisper.\n\nTente manualmente:\n  pip install --user faster-whisper\n  ou: pipx install faster-whisper\n\nDetalhe: {}",
        stderr.lines().take(5).collect::<Vec<_>>().join("\n")
    ))
}

#[tauri::command]
async fn install_ffmpeg(app: tauri::AppHandle) -> Result<String, String> {
    let script_path = app
        .path()
        .resolve("resources/install_ffmpeg.py", BaseDirectory::Resource)
        .map_err(|e| format!("Script install_ffmpeg.py não encontrado no app: {:?}", e))?;
    let transcribe_script = app
        .path()
        .resolve("resources/transcribe.py", BaseDirectory::Resource)
        .ok();
    let python_bin = resolve_python(transcribe_script.as_deref());
    let install_dir = local_ffmpeg_bin_dir()
        .ok_or_else(|| "Não foi possível localizar a pasta de dados do app".to_string())?;
    let script_path = script_path
        .to_str()
        .ok_or_else(|| "Path do script inválido (encoding)".to_string())?
        .to_string();
    let install_dir = install_dir
        .to_str()
        .ok_or_else(|| "Path de instalação inválido (encoding)".to_string())?
        .to_string();

    tokio::task::spawn_blocking(move || install_ffmpeg_blocking(&python_bin, &script_path, &install_dir))
        .await
        .map_err(|e| format!("Erro interno: {}", e))?
}

fn install_ffmpeg_blocking(
    python_bin: &str,
    script_path: &str,
    install_dir: &str,
) -> Result<String, String> {
    let python_ok = python_command(python_bin)
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

    let output = python_command(python_bin)
        .arg(script_path)
        .arg(install_dir)
        .output()
        .map_err(|e| format!("Falha ao executar instalador do FFmpeg: {}", e))?;

    if output.status.success() {
        return Ok("FFmpeg instalado com sucesso!".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "Falha ao instalar FFmpeg.\n\nDetalhe: {}\n{}",
        stdout.lines().take(5).collect::<Vec<_>>().join("\n"),
        stderr.lines().take(5).collect::<Vec<_>>().join("\n")
    ))
}

#[tauri::command]
fn get_video_duration(path: String) -> Result<f64, String> {
    let ffprobe_bin = resolve_command("ffprobe");
    let output = Command::new(&ffprobe_bin)
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
    let ffmpeg_bin = resolve_command("ffmpeg");
    let ffmpeg_out = Command::new(&ffmpeg_bin)
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
    let python_bin = resolve_python(Some(&script_path));

    let mut child = python_command(&python_bin)
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
            install_ffmpeg,
            get_cpu_count,
            get_video_duration,
            cancel_transcription
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
