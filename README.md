# Transcribe

App desktop para transcrever videos usando inteligencia artificial (Whisper). Construido com Tauri (Rust) + React + Python (faster-whisper).

![Tauri](https://img.shields.io/badge/Tauri-2-blue)
![License](https://img.shields.io/badge/license-MIT-green)

## Features

- **Transcricao local** — tudo roda na sua maquina, sem enviar dados para nuvem
- **Modelos Whisper** — escolha entre tiny, base, small, medium, large e turbo
- **Progresso em tempo real** — barra de progresso + texto aparecendo conforme transcreve
- **Multi-idioma** — interface em Portugues (BR), Ingles e Espanhol
- **Controle de CPU threads** — ajuste quantos cores usar na transcricao
- **Cancelamento** — cancele a qualquer momento e veja o resultado parcial
- **Verificacao de dependencias** — o app verifica e ajuda a instalar o que falta

## Requisitos 

| Dependencia | Como instalar |
|---|---|
| **FFmpeg** | `sudo apt install ffmpeg` (Linux) / `brew install ffmpeg` (macOS) / [ffmpeg.org](https://ffmpeg.org/download.html) (Windows) |
| **Python 3** | `sudo apt install python3` (Linux) / `brew install python3` (macOS) / [python.org](https://python.org/downloads) (Windows) |
| **faster-whisper** | `pip install faster-whisper` |

O app verifica essas dependencias ao abrir e oferece instalar o `faster-whisper` automaticamente.

## Instalacao

Baixe o instalador para seu sistema na pagina de [Releases](../../releases):

| Plataforma | Formato |
|---|---|
| Linux | `.deb`, `.rpm`, `.AppImage` |
| Windows | `.msi`, `.exe` |
| macOS | `.dmg` |

## Desenvolvimento

```bash
# Clone o repositorio
git clone <repo-url>
cd transcribe-app

# Instale as dependencias do frontend
pnpm install

# (Opcional) Configure o Python com faster-whisper
python3 -m venv .venv
source .venv/bin/activate
pip install faster-whisper

# Rode em modo dev
pnpm tauri dev
```

## Build

```bash
pnpm tauri build
```

Os instaladores serao gerados em `src-tauri/target/release/bundle/`.

## Releases automaticas

O projeto usa GitHub Actions para gerar builds automaticamente. Para criar uma release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Isso dispara o workflow que builda para Linux, Windows e macOS e cria a release no GitHub com os instaladores.

## Arquitetura

```
transcribe-app/
├── src/                     # Frontend React + TypeScript
│   ├── components/          # VideoUploader, TranscriptionResult, etc.
│   ├── i18n.ts              # Sistema de traducoes
│   └── App.tsx              # Componente principal
├── src-tauri/
│   ├── src/lib.rs           # Backend Rust (ffmpeg + Python orchestration)
│   └── resources/
│       └── transcribe.py    # Script de transcricao (faster-whisper)
└── .github/workflows/
    └── release.yml          # CI/CD para releases multi-plataforma
```

**Fluxo:** Video → FFmpeg (audio MP3) → Python/Whisper (transcricao) → Texto

O Rust orquestra o processo, emitindo eventos em tempo real para o frontend mostrar o progresso.
