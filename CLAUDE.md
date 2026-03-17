# Transcribe App

App desktop para transcrever videos usando IA (faster-whisper). Tauri 2 (Rust) + React + Python.

## Comandos

```bash
pnpm install          # instalar dependencias frontend
pnpm tauri dev        # rodar em modo dev
pnpm tauri build      # build completo (todos os targets)
pnpm build:linux      # build linux (deb, rpm, appimage)
pnpm build:windows    # build windows (msi, nsis)
pnpm build:mac        # build mac (dmg)
cargo check           # verificar compilacao do Rust (rodar dentro de src-tauri/)
```

## Arquitetura

```
Video -> FFmpeg (extrai audio MP3) -> Python/faster-whisper (transcreve) -> Texto
```

O Rust orquestra o processo, emitindo eventos em tempo real para o frontend via Tauri events.

### Backend (Rust) - `src-tauri/src/lib.rs`

**Tauri commands (invocados pelo frontend via `invoke()`):**
- `transcribe_video(path, model, threads)` - async, roda em `spawn_blocking`
- `check_dependencies()` - verifica ffmpeg, python3, faster-whisper
- `install_dependencies()` - async, tenta pip --user, --break-system-packages, pipx
- `get_video_duration(path)` - usa ffprobe
- `get_cpu_count()` - retorna threads disponiveis
- `cancel_transcription()` - mata o processo Python via `child.kill()`

**Funcoes auxiliares importantes:**
- `resolve_command(name)` - busca binarios em paths comuns (/opt/homebrew/bin, /usr/local/bin, etc). Necessario porque apps GUI no macOS nao herdam o PATH do terminal. No AppImage, pula binarios embutidos via `canonicalize()`.
- `python_command(bin)` - cria Command limpando PYTHONHOME/PYTHONPATH dentro de AppImage (o AppImage seta essas vars apontando pro filesystem read-only interno).
- `resolve_python(script_path)` - tenta venv do backend primeiro, depois `resolve_command("python3")`.
- `backend_venv_python(script_path)` - procura `backend/.venv/bin/python3` subindo ate 8 niveis de diretorio.

**Estado compartilhado:**
- `TranscribeProcess(Arc<Mutex<Option<Child>>>)` - armazena o processo Python para cancel imediato.

**Protocolo Python->Rust (stdout streaming):**
- `__DURATION__:<float>` - duracao total do audio
- `__LANG__:<string>` - idioma detectado
- `__SEG__:<end_time>|<text>` - segmento transcrito (usado pra barra de progresso)
- `__DONE__` - transcricao concluida

**Cancelamento:**
- Retorna `Err("__CANCELLED__<texto_parcial>")` - frontend detecta o prefixo e exibe resultado parcial.

### Frontend (React) - `src/`

**Componentes:**
- `App.tsx` - componente principal, state machine: checking -> missing-deps -> idle -> loading -> done -> error
- `VideoUploader.tsx` - upload de video, grid de modelos (tiny/base/small/medium/large/turbo), opcoes avancadas (CPU threads slider), duracao do video
- `TranscriptionResult.tsx` - resultado final, aceita `isPartial` para cancelamento
- `DependencyCheck.tsx` - verifica/instala dependencias, spinner durante instalacao
- `SettingsDropdown.tsx` - fixo no canto inferior direito, dropdown abre pra cima, seletor de idioma

**i18n:**
- `i18n.ts` - strings traduzidas (pt-BR, en, es)
- `LangContext.ts` - React Context com `lang`, `setLang`, `t(key)`
- Idioma salvo em `localStorage`

**Eventos Tauri (listen):**
- `transcribe-step` - muda o passo atual (audio/text)
- `transcribe-progress` - atualiza barra de progresso + texto em tempo real
- `transcribe-lang` - idioma detectado pelo whisper

### Python - `src-tauri/resources/transcribe.py`

- Recebe: arquivo mp3, nome, flags `--model` e `--threads`
- Usa `faster_whisper.WhisperModel`
- Output via protocolo stdout (flush=True para streaming)
- Salva resultado em `./output-text/<nome>.txt`

## Configuracao Tauri - `src-tauri/tauri.conf.json`

- Bundle resources: `resources/transcribe.py`
- Icons: `icons/` (32, 128, 128@2x, icns, ico)
- Identifier: `com.srwalkerb.transcribe-app`

## CI/CD - `.github/workflows/release.yml`

- Trigger: push de tags `v*`
- **Precisa de** `permissions: contents: write` (sem isso da "Resource not accessible by integration")
- Matrix: ubuntu-22.04, windows-latest
- Usa `tauri-apps/tauri-action@v0`
- Para criar release: `git tag v0.x.x && git push origin v0.x.x`

## Problemas conhecidos e solucoes

**AppImage + Python:** O AppImage embute seu proprio Python e seta PYTHONHOME/PYTHONPATH apontando pro filesystem read-only interno. Solucao: `python_command()` limpa essas vars, `resolve_command()` usa `canonicalize()` pra pular binarios do mount do AppImage.

**Linux externally-managed-environment:** Distros modernas bloqueiam `pip install` global. Solucao: tenta `--user` primeiro, depois `--break-system-packages`, depois `pipx`.

**macOS PATH nao herdado:** Apps GUI no macOS nao recebem o PATH do terminal. Solucao: `resolve_command()` busca em `/opt/homebrew/bin`, `/usr/local/bin`, `/usr/bin`.

**macOS code signing:** Builds nao sao assinados/notarizados. O Gatekeeper bloqueia o app na primeira abertura. Usuario precisa ir em System Settings > Privacy & Security para permitir.
