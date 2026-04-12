use std::fs;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use chrono::Local;
use serde::Serialize;
use tauri::path::BaseDirectory;
use tauri::{Emitter, Manager};

#[derive(Clone, Serialize)]
struct InstallProgress {
    target: String,
    step: String,
    progress: f32,
}

fn emit_progress(app: &tauri::AppHandle, target: &str, step: &str, progress: f32) {
    let _ = app.emit(
        "install-progress",
        InstallProgress {
            target: target.to_string(),
            step: step.to_string(),
            progress,
        },
    );
}

#[cfg(windows)]
fn suppress_console(cmd: &mut Command) {
    // Evita a janela de terminal "aparecer/fechar" ao executar binários de CLI no Windows.
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
}

#[cfg(not(windows))]
fn suppress_console(_cmd: &mut Command) {}

/// Check if we're running inside an AppImage.
fn is_appimage() -> bool {
    std::env::var("APPIMAGE").is_ok()
}

/// Local folder used to store downloaded FFmpeg binaries.
fn local_ffmpeg_bin_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|base| base.join("transcribe-app").join("ffmpeg").join("bin"))
}

/// App-managed uv binary directory (Windows only).
#[cfg(windows)]
fn local_uv_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|base| base.join("transcribe-app").join("uv"))
}

#[cfg(windows)]
fn local_uv_exe() -> Option<String> {
    let exe = local_uv_dir()?.join("uv.exe");
    if exe.is_file() {
        Some(exe.to_string_lossy().to_string())
    } else {
        None
    }
}

/// App-managed Python venv directory (Windows only).
/// Contains a self-contained Python 3.12 runtime (via uv) with faster-whisper.
#[cfg(windows)]
fn local_runtime_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|base| base.join("transcribe-app").join("runtime"))
}

#[cfg(windows)]
fn local_runtime_python() -> Option<String> {
    let exe = local_runtime_dir()?.join("Scripts").join("python.exe");
    if exe.is_file() {
        Some(exe.to_string_lossy().to_string())
    } else {
        None
    }
}

/// Where uv stores the downloaded Python interpreters.
#[cfg(windows)]
fn local_python_install_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|base| base.join("transcribe-app").join("python"))
}

#[cfg(windows)]
fn is_local_runtime_python(path: &str) -> bool {
    match local_runtime_dir() {
        Some(dir) => {
            let dir_str = dir.to_string_lossy().to_lowercase();
            path.to_lowercase().starts_with(&dir_str)
        }
        None => false,
    }
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

/// Download `uv.exe` from GitHub releases into `local_uv_dir()`.
#[cfg(windows)]
fn download_uv_windows() -> Result<String, String> {
    let uv_dir = local_uv_dir().ok_or_else(|| "Pasta de dados indisponível".to_string())?;
    let uv_dir_str = uv_dir.to_string_lossy().replace('\'', "''");
    let ps_script = format!(
        "$ProgressPreference='SilentlyContinue'; $ErrorActionPreference='Stop'; \
         [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
         $url = 'https://github.com/astral-sh/uv/releases/latest/download/uv-x86_64-pc-windows-msvc.zip'; \
         $tmp = Join-Path $env:TEMP ('uv-dl-' + [guid]::NewGuid() + '.zip'); \
         $ext = Join-Path $env:TEMP ('uv-ext-' + [guid]::NewGuid()); \
         Invoke-WebRequest -Uri $url -OutFile $tmp -UseBasicParsing; \
         Expand-Archive -Path $tmp -DestinationPath $ext -Force; \
         New-Item -ItemType Directory -Force -Path '{dir}' | Out-Null; \
         Get-ChildItem -Path $ext -Recurse -Include uv.exe,uvx.exe | ForEach-Object {{ Copy-Item $_.FullName -Destination '{dir}' -Force }}; \
         Remove-Item $tmp,$ext -Recurse -Force -ErrorAction SilentlyContinue; \
         if (-not (Test-Path (Join-Path '{dir}' 'uv.exe'))) {{ throw 'uv.exe não encontrado após extração' }}",
        dir = uv_dir_str
    );
    let mut cmd = Command::new("powershell");
    suppress_console(&mut cmd);
    let out = cmd
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps_script])
        .output()
        .map_err(|e| format!("Falha ao executar PowerShell para baixar uv: {}", e))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "Falha ao baixar o uv:\n{}",
            stderr.lines().take(8).collect::<Vec<_>>().join("\n")
        ));
    }
    local_uv_exe().ok_or_else(|| "uv.exe não foi instalado".to_string())
}

#[cfg(windows)]
fn ensure_uv_windows() -> Result<String, String> {
    if let Some(p) = local_uv_exe() {
        return Ok(p);
    }
    download_uv_windows()
}

/// Create a self-contained Python 3.12 venv via uv, auto-downloading Python if needed.
#[cfg(windows)]
fn install_python_via_uv(app: &tauri::AppHandle) -> Result<String, String> {
    emit_progress(app, "python", "Baixando gerenciador uv...", 0.1);
    let uv = ensure_uv_windows()?;
    let runtime = local_runtime_dir().ok_or_else(|| "Pasta de runtime indisponível".to_string())?;
    let python_install = local_python_install_dir().ok_or_else(|| "Pasta de Python indisponível".to_string())?;

    if let Some(parent) = runtime.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::create_dir_all(&python_install);

    emit_progress(app, "python", "Baixando Python 3.12 portátil...", 0.3);
    let runtime_str = runtime.to_string_lossy().to_string();
    let mut cmd = Command::new(&uv);
    suppress_console(&mut cmd);
    let out = cmd
        .env("UV_PYTHON_INSTALL_DIR", python_install.as_os_str())
        .env("UV_PYTHON_PREFERENCE", "only-managed")
        .args(["venv", "--python", "3.12", "--seed", &runtime_str])
        .output()
        .map_err(|e| format!("Falha ao executar uv venv: {}", e))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        return Err(format!(
            "uv venv falhou.\n\nstdout:\n{}\n\nstderr:\n{}",
            stdout.lines().take(8).collect::<Vec<_>>().join("\n"),
            stderr.lines().take(8).collect::<Vec<_>>().join("\n")
        ));
    }

    emit_progress(app, "python", "Finalizando...", 0.9);
    local_runtime_python().ok_or_else(|| {
        "Venv criada mas python.exe não foi encontrado em runtime\\Scripts".to_string()
    })
}

/// Query the Windows registry via PowerShell for Pythons installed per PEP 514.
/// This catches Pythons that aren't on PATH and weren't installed with the py launcher.
#[cfg(windows)]
fn python_from_registry() -> Option<String> {
    let ps_script = r#"
        $ProgressPreference='SilentlyContinue'; $ErrorActionPreference='SilentlyContinue';
        $roots = @(
            'HKCU:\Software\Python\PythonCore',
            'HKLM:\Software\Python\PythonCore',
            'HKLM:\Software\WOW6432Node\Python\PythonCore'
        );
        foreach ($root in $roots) {
            if (Test-Path $root) {
                Get-ChildItem $root | ForEach-Object {
                    $ip = Join-Path $_.PSPath 'InstallPath';
                    if (Test-Path $ip) {
                        $prop = Get-ItemProperty $ip;
                        $dir = $prop.'(default)';
                        if (-not $dir) { $dir = $prop.'ExecutablePath' | Split-Path -Parent }
                        if ($dir) {
                            $exe = Join-Path $dir 'python.exe';
                            if (Test-Path $exe) { Write-Output $exe }
                        }
                    }
                }
            }
        }
    "#;
    let mut cmd = Command::new("powershell");
    suppress_console(&mut cmd);
    let output = cmd
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps_script])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let path = line.trim();
        if path.to_lowercase().ends_with("python.exe") && Path::new(path).is_file() {
            return Some(path.to_string());
        }
    }
    None
}

/// Use `where.exe` to locate a command in PATH — works even when we can't inherit it.
#[cfg(windows)]
fn python_from_where() -> Option<String> {
    for name in &["python.exe", "python3.exe"] {
        let mut cmd = Command::new("where");
        suppress_console(&mut cmd);
        if let Ok(output) = cmd.arg(name).output() {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let path = line.trim();
                    // Pular o stub da Microsoft Store (WindowsApps) que só redireciona pra store.
                    if path.to_lowercase().contains("\\windowsapps\\") {
                        continue;
                    }
                    if Path::new(path).is_file() {
                        return Some(path.to_string());
                    }
                }
            }
        }
    }
    None
}

#[cfg(windows)]
fn python_from_py_launcher() -> Option<String> {
    let mut cmd = Command::new("py");
    suppress_console(&mut cmd);
    let output = cmd.arg("-0p").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    // Formato típico: " -V:3.12 *        C:\...\python.exe"
    // Procuramos linhas que terminam em python.exe e extraímos a partir da letra de drive.
    for line in text.lines() {
        let trimmed = line.trim_end();
        if !trimmed.to_lowercase().ends_with("python.exe") {
            continue;
        }
        // Localiza a primeira ocorrência de "X:\" onde X é letra (drive).
        let bytes = trimmed.as_bytes();
        for i in 0..bytes.len().saturating_sub(2) {
            if bytes[i].is_ascii_alphabetic() && bytes[i + 1] == b':' && bytes[i + 2] == b'\\' {
                let path = &trimmed[i..];
                if Path::new(path).is_file() {
                    return Some(path.to_string());
                }
                break;
            }
        }
    }
    None
}

fn resolve_command_any(names: &[&str]) -> String {
    for name in names {
        if let Some(installed) = installed_command_path(name) {
            return installed;
        }
    }

    #[cfg(windows)]
    {
        // Para python, tentar descobrir dinamicamente, nesta ordem:
        //  1. Runtime próprio do app (venv criada via uv) — caminho padrão.
        //  2. Registro do Windows (PEP 514) — Python instalado pelo usuário.
        //  3. Launcher py.exe.
        //  4. where.exe / scan de diretórios (feito abaixo).
        let wants_python = names.iter().any(|n| *n == "python" || *n == "python3");
        if wants_python {
            if let Some(p) = local_runtime_python() {
                return p;
            }
            if let Some(p) = python_from_registry() {
                return p;
            }
            if let Some(p) = python_from_py_launcher() {
                return p;
            }
            if let Some(p) = python_from_where() {
                return p;
            }
        }

        let mut win_dirs: Vec<String> = Vec::new();
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            for ver in ["314", "313", "312", "311", "310"] {
                win_dirs.push(format!("{}\\Programs\\Python\\Python{}", local, ver));
                win_dirs.push(format!("{}\\Programs\\Python\\Python{}\\Scripts", local, ver));
            }
            win_dirs.push(format!("{}\\Microsoft\\WindowsApps", local));
        }
        for env_var in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Ok(pf) = std::env::var(env_var) {
                for ver in ["314", "313", "312", "311", "310"] {
                    win_dirs.push(format!("{}\\Python{}", pf, ver));
                    win_dirs.push(format!("{}\\Python{}\\Scripts", pf, ver));
                }
            }
        }
        for name in names {
            for dir in &win_dirs {
                let full = format!("{}\\{}.exe", dir, name);
                if Path::new(&full).is_file() {
                    return full;
                }
            }
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
    cmd.env("PYTHONIOENCODING", "utf-8");
    cmd.env("HF_HUB_DISABLE_SYMLINKS_WARNING", "1");
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

/// Shared state: holds the running Python child process + cancellation flag.
struct TranscribeState {
    child: Option<Child>,
    cancel_requested: bool,
}

/// Shared state wrapper for Tauri.
struct TranscribeProcess(Arc<Mutex<TranscribeState>>);

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

    emit_progress(&app, "whisper", "Instalando faster-whisper...", -1.0);
    let result = tokio::task::spawn_blocking(move || install_dependencies_blocking(&python_bin))
        .await
        .map_err(|e| format!("Erro interno: {}", e))?;
    if result.is_ok() {
        emit_progress(&app, "whisper", "Concluído!", 1.0);
    }
    result
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

    // Strategy 0 (Windows): runtime próprio do app — usar uv pip install (rápido) ou pip sem --user.
    #[cfg(windows)]
    {
        if is_local_runtime_python(python_bin) {
            // Preferir uv pip se disponível (mais rápido e resolve deps melhor).
            if let Some(uv) = local_uv_exe() {
                let mut cmd = Command::new(&uv);
                suppress_console(&mut cmd);
                let out = cmd
                    .args(["pip", "install", "--python", python_bin, "faster-whisper"])
                    .output()
                    .map_err(|e| format!("Falha ao executar uv pip: {}", e))?;
                if out.status.success() {
                    return Ok("faster-whisper instalado no runtime do app (via uv)!".to_string());
                }
                let stderr = String::from_utf8_lossy(&out.stderr);
                return Err(format!(
                    "Falha ao instalar faster-whisper via uv pip:\n{}",
                    stderr.lines().take(8).collect::<Vec<_>>().join("\n")
                ));
            }
            // Sem uv (improvável): pip direto dentro da venv, sem --user.
            let out = python_command(python_bin)
                .args(["-m", "pip", "install", "faster-whisper"])
                .output()
                .map_err(|e| format!("Falha ao executar pip: {}", e))?;
            if out.status.success() {
                return Ok("faster-whisper instalado no runtime do app!".to_string());
            }
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(format!(
                "Falha ao instalar faster-whisper:\n{}",
                stderr.lines().take(8).collect::<Vec<_>>().join("\n")
            ));
        }
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
fn get_platform() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

#[tauri::command]
async fn install_python(app: tauri::AppHandle) -> Result<String, String> {
    tokio::task::spawn_blocking(move || install_python_blocking(&app))
        .await
        .map_err(|e| format!("Erro interno: {}", e))?
}

#[cfg(windows)]
fn detect_python_windows() -> Option<String> {
    let bin = resolve_command_any(&["python3", "python"]);
    let mut cmd = python_command(&bin);
    suppress_console(&mut cmd);
    let ok = cmd
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok { Some(bin) } else { None }
}

#[cfg(windows)]
fn install_python_blocking(app: &tauri::AppHandle) -> Result<String, String> {
    emit_progress(app, "python", "Verificando instalações existentes...", -1.0);
    if let Some(path) = detect_python_windows() {
        return Ok(format!("Python já instalado em {}.", path));
    }

    match install_python_via_uv(app) {
        Ok(path) => Ok(format!("Python portátil instalado em {}.", path)),
        Err(e) => Err(format!(
            "Falha ao preparar o Python portátil via uv.\n\n{}\n\nSe o problema persistir, baixe o Python manualmente em https://www.python.org/downloads/ marcando 'Add python.exe to PATH'.",
            e
        )),
    }
}

#[cfg(not(windows))]
fn install_python_blocking(_app: &tauri::AppHandle) -> Result<String, String> {
    Err("Instalação automática do Python só é suportada no Windows.".into())
}

#[tauri::command]
async fn install_ffmpeg(app: tauri::AppHandle) -> Result<String, String> {
    let install_dir = local_ffmpeg_bin_dir()
        .ok_or_else(|| "Não foi possível localizar a pasta de dados do app".to_string())?;
    let install_dir = install_dir
        .to_str()
        .ok_or_else(|| "Path de instalação inválido (encoding)".to_string())?
        .to_string();

    #[cfg(windows)]
    {
        let dir = install_dir.clone();
        let a = app.clone();
        return tokio::task::spawn_blocking(move || install_ffmpeg_windows_native(&a, &dir))
            .await
            .map_err(|e| format!("Erro interno: {}", e))?;
    }

    #[cfg(not(windows))]
    {
        let script_path = app
            .path()
            .resolve("resources/install_ffmpeg.py", BaseDirectory::Resource)
            .map_err(|e| format!("Script install_ffmpeg.py não encontrado no app: {:?}", e))?;
        let transcribe_script = app
            .path()
            .resolve("resources/transcribe.py", BaseDirectory::Resource)
            .ok();
        let python_bin = resolve_python(transcribe_script.as_deref());
        let script_path = script_path
            .to_str()
            .ok_or_else(|| "Path do script inválido (encoding)".to_string())?
            .to_string();

        emit_progress(&app, "ffmpeg", "Instalando FFmpeg...", -1.0);
        return tokio::task::spawn_blocking(move || {
            install_ffmpeg_blocking(&python_bin, &script_path, &install_dir)
        })
        .await
        .map_err(|e| format!("Erro interno: {}", e))?;
    }
}

#[cfg(windows)]
fn run_ps(script: &str) -> Result<String, String> {
    let mut cmd = Command::new("powershell");
    suppress_console(&mut cmd);
    let out = cmd
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
        .output()
        .map_err(|e| format!("Falha ao executar PowerShell: {}", e))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        Err(stderr.lines().take(8).collect::<Vec<_>>().join("\n"))
    }
}

#[cfg(windows)]
fn install_ffmpeg_windows_native(app: &tauri::AppHandle, install_dir: &str) -> Result<String, String> {
    let url = "https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-win64-gpl.zip";
    let escaped_dir = install_dir.replace('\'', "''");

    // Stage 1: Download
    emit_progress(app, "ffmpeg", "Baixando FFmpeg (~80 MB)...", 0.1);
    let tmp_var = format!(
        "$ProgressPreference='SilentlyContinue'; $ErrorActionPreference='Stop'; \
         [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
         $tmp = Join-Path $env:TEMP ('ffmpeg-' + [guid]::NewGuid() + '.zip'); \
         Invoke-WebRequest -Uri '{url}' -OutFile $tmp -UseBasicParsing; \
         Write-Output $tmp",
        url = url
    );
    let zip_path = run_ps(&tmp_var)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("Falha ao baixar FFmpeg:\n{}", e))?;

    // Stage 2: Extract
    emit_progress(app, "ffmpeg", "Extraindo arquivos...", 0.6);
    let extract_script = format!(
        "$ProgressPreference='SilentlyContinue'; $ErrorActionPreference='Stop'; \
         $dst = Join-Path $env:TEMP ('ffmpeg-extract-' + [guid]::NewGuid()); \
         Expand-Archive -Path '{zip}' -DestinationPath $dst -Force; \
         $bins = Get-ChildItem -Path $dst -Recurse -Include ffmpeg.exe,ffprobe.exe; \
         if ($bins.Count -eq 0) {{ throw 'Binários ffmpeg/ffprobe não encontrados no zip' }}; \
         New-Item -ItemType Directory -Force -Path '{dir}' | Out-Null; \
         foreach ($b in $bins) {{ Copy-Item $b.FullName -Destination '{dir}' -Force }}; \
         Remove-Item '{zip}',$dst -Recurse -Force -ErrorAction SilentlyContinue",
        zip = zip_path.replace('\'', "''"),
        dir = escaped_dir
    );
    run_ps(&extract_script).map_err(|e| format!("Falha ao extrair FFmpeg:\n{}", e))?;

    // Stage 3: Done
    emit_progress(app, "ffmpeg", "Concluído!", 1.0);
    Ok("FFmpeg instalado com sucesso!".to_string())
}

#[cfg(not(windows))]
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
        guard.child = None;
        guard.cancel_requested = false;
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
    guard.cancel_requested = true;
    if let Some(ref mut child) = guard.child {
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
    process_state: &Mutex<TranscribeState>,
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
    let mut ffmpeg_cmd = Command::new(&ffmpeg_bin);
    suppress_console(&mut ffmpeg_cmd);
    let ffmpeg_out = ffmpeg_cmd
        .arg("-i")
        .arg(path)
        .arg("-b:a")
        .arg("128k")
        .arg("-loglevel")
        .arg("error")
        .arg("-hide_banner")
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

    let mut py_cmd = python_command(&python_bin);
    suppress_console(&mut py_cmd);

    let mut child = py_cmd
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
        guard.child = Some(child);
    }

    // Drena stderr em paralelo para evitar bloqueios e para termos detalhes caso falhe.
    let stderr_handle = std::thread::spawn(move || {
        let mut lines: VecDeque<String> = VecDeque::new();
        const MAX_LINES: usize = 250;

        if let Some(stderr) = stderr_pipe {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                let l = match line {
                    Ok(s) => s,
                    Err(_) => break,
                };
                lines.push_back(l);
                while lines.len() > MAX_LINES {
                    lines.pop_front();
                }
            }
        }

        lines.into_iter().collect::<Vec<_>>().join("\n")
    });

    let reader = BufReader::new(stdout);
    let mut duration: f64 = 0.0;
    let mut full_text = String::new();
    let mut done_received = false;

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
            done_received = true;
            break;
        }
    }

    // Wait for process to finish and check result
    let mut guard = process_state.lock().unwrap();
    let cancel_requested = guard.cancel_requested;
    let status_success = if let Some(ref mut child) = guard.child {
        match child.wait() {
            Ok(s) => s.success(),
            Err(_) => false,
        }
    } else {
        // child was already taken/killed
        false
    };
    // Clear the process from state (and reset cancel flag for next run)
    guard.child = None;
    guard.cancel_requested = false;
    drop(guard);

    let stderr_output = stderr_handle.join().unwrap_or_default();

    // Classificação:
    // - Se o usuário pediu cancelamento, retornamos "__CANCELLED__" (mesmo que tenha falhado).
    // - Se não cancelou e falhou, retornamos erro real com detalhes para depuração.
    if !status_success {
        if cancel_requested && !done_received {
            // Mantém o protocolo do frontend: quando começa com "__CANCELLED__", ele exibe parcial.
            if !full_text.trim().is_empty() {
                return Err(format!("__CANCELLED__{}", full_text));
            }
            return Err("__CANCELLED__".to_string());
        }

        let partial_hint = if full_text.trim().is_empty() {
            "".to_string()
        } else {
            format!("\n\nTrecho transcrito (parcial):\n{}", full_text)
        };

        if stderr_output.contains("faster_whisper")
            || stderr_output.contains("ModuleNotFoundError")
            || stderr_output.contains("No module named")
        {
            return Err(
                format!(
                    "Para transcrever, é necessário ter o módulo faster-whisper instalado.\n\n\
Volte para a tela inicial e clique em \"Verificar dependências\" para instalar automaticamente.\n\n\
Detalhe (stderr):\n{}{}",
                    stderr_output.trim(),
                    partial_hint
                )
            );
        } else if stderr_output.contains("python3")
            && (stderr_output.contains("not found") || stderr_output.contains("No such file"))
        {
            return Err(
                format!(
                    "Python não foi encontrado no sistema. Instale Python 3 e o módulo faster-whisper.\n\n\
Detalhe (stderr):\n{}{}",
                    stderr_output.trim(),
                    partial_hint
                )
            );
        }

        return Err(format!(
            "Falha ao transcrever o vídeo.\n\nDetalhe (stderr):\n{}{}",
            stderr_output.trim(),
            partial_hint
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(TranscribeProcess(Arc::new(Mutex::new(TranscribeState {
            child: None,
            cancel_requested: false,
        }))))
        .invoke_handler(tauri::generate_handler![
            transcribe_video,
            check_dependencies,
            install_dependencies,
            install_ffmpeg,
            install_python,
            get_platform,
            get_cpu_count,
            get_video_duration,
            cancel_transcription
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
