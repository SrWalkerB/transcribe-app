export type Lang = "pt-BR" | "en" | "es";

export const LANGUAGES: { value: Lang; label: string }[] = [
  { value: "pt-BR", label: "Português (BR)" },
  { value: "en", label: "English" },
  { value: "es", label: "Español" },
];

const translations = {
  // Header
  "app.subtitle": {
    "pt-BR": "Transcreva seus vídeos de forma simples e rápida",
    en: "Transcribe your videos simply and quickly",
    es: "Transcribe tus videos de forma simple y rapida",
  },

  // Dependencies
  "deps.title": {
    "pt-BR": "Dependências necessárias",
    en: "Required dependencies",
    es: "Dependencias necesarias",
  },
  "deps.subtitle": {
    "pt-BR": "Antes de transcrever, verifique se as dependências estão instaladas:",
    en: "Before transcribing, check if the dependencies are installed:",
    es: "Antes de transcribir, verifica si las dependencias estan instaladas:",
  },
  "deps.ffmpeg.hint": {
    "pt-BR": "Clique em instalar para baixar a versão certa para seu sistema",
    en: "Click install to download the right build for your system",
    es: "Haz clic en instalar para descargar la versión correcta para tu sistema",
  },
  "deps.ffmpeg.install": {
    "pt-BR": "Instalar FFmpeg",
    en: "Install FFmpeg",
    es: "Instalar FFmpeg",
  },
  "deps.ffmpeg.installing": {
    "pt-BR": "Instalando FFmpeg...",
    en: "Installing FFmpeg...",
    es: "Instalando FFmpeg...",
  },
  "deps.python.hint.windows": {
    "pt-BR": "Clique em instalar para baixar o Python 3.12 via winget",
    en: "Click install to download Python 3.12 via winget",
    es: "Haz clic en instalar para descargar Python 3.12 via winget",
  },
  "deps.python.hint.macos": {
    "pt-BR": "Instale via python.org ou `brew install python3`",
    en: "Install via python.org or `brew install python3`",
    es: "Instala via python.org o `brew install python3`",
  },
  "deps.python.hint.linux": {
    "pt-BR": "Instale via gerenciador de pacotes (ex: sudo apt install python3)",
    en: "Install via package manager (e.g.: sudo apt install python3)",
    es: "Instala via gestor de paquetes (ej: sudo apt install python3)",
  },
  "deps.python.install": {
    "pt-BR": "Instalar Python",
    en: "Install Python",
    es: "Instalar Python",
  },
  "deps.python.installing": {
    "pt-BR": "Instalando Python...",
    en: "Installing Python...",
    es: "Instalando Python...",
  },
  "deps.whisper.hint": {
    "pt-BR": "Módulo Python para transcrição de áudio",
    en: "Python module for audio transcription",
    es: "Modulo Python para transcripcion de audio",
  },
  "deps.whisper.needPython": {
    "pt-BR": "Instale Python 3 primeiro",
    en: "Install Python 3 first",
    es: "Instala Python 3 primero",
  },
  "deps.installing": {
    "pt-BR": "Instalando faster-whisper...",
    en: "Installing faster-whisper...",
    es: "Instalando faster-whisper...",
  },
  "deps.install": {
    "pt-BR": "Instalar faster-whisper",
    en: "Install faster-whisper",
    es: "Instalar faster-whisper",
  },
  "deps.recheck": {
    "pt-BR": "Verificar novamente",
    en: "Check again",
    es: "Verificar de nuevo",
  },
  "deps.checking": {
    "pt-BR": "Verificando dependências...",
    en: "Checking dependencies...",
    es: "Verificando dependencias...",
  },

  // Uploader
  "upload.dragOrBrowse": {
    "pt-BR": "Arraste seu vídeo aqui ou",
    en: "Drag your video here or",
    es: "Arrastra tu video aqui o",
  },
  "upload.browse": {
    "pt-BR": "clique para buscar",
    en: "click to browse",
    es: "haz clic para buscar",
  },
  "upload.formats": {
    "pt-BR": "Formatos aceitos: MP4, MKV, AVI, MOV, WEBM",
    en: "Accepted formats: MP4, MKV, AVI, MOV, WEBM",
    es: "Formatos aceptados: MP4, MKV, AVI, MOV, WEBM",
  },
  "upload.dropWarning": {
    "pt-BR": 'Use "clique para buscar" para selecionar o vídeo e transcrever (arrastar não envia o caminho do arquivo).',
    en: 'Use "click to browse" to select the video and transcribe (drag does not send the file path).',
    es: 'Usa "haz clic para buscar" para seleccionar el video y transcribir (arrastrar no envia la ruta del archivo).',
  },
  "upload.modelLabel": {
    "pt-BR": "Modelo de transcrição",
    en: "Transcription model",
    es: "Modelo de transcripcion",
  },
  "upload.advanced": {
    "pt-BR": "Opções avançadas",
    en: "Advanced options",
    es: "Opciones avanzadas",
  },
  "upload.transcribe": {
    "pt-BR": "Transcrever",
    en: "Transcribe",
    es: "Transcribir",
  },
  "upload.transcribing": {
    "pt-BR": "Transcrevendo...",
    en: "Transcribing...",
    es: "Transcribiendo...",
  },
  "upload.removeFile": {
    "pt-BR": "Remover arquivo",
    en: "Remove file",
    es: "Eliminar archivo",
  },

  // Model descriptions
  "model.tiny": {
    "pt-BR": "Mais rápido, menor precisão",
    en: "Fastest, lowest accuracy",
    es: "Mas rapido, menor precision",
  },
  "model.base": {
    "pt-BR": "Bom equilíbrio velocidade/qualidade",
    en: "Good speed/quality balance",
    es: "Buen equilibrio velocidad/calidad",
  },
  "model.small": {
    "pt-BR": "Melhor precisão, mais lento",
    en: "Better accuracy, slower",
    es: "Mejor precision, mas lento",
  },
  "model.medium": {
    "pt-BR": "Alta precisão, lento",
    en: "High accuracy, slow",
    es: "Alta precision, lento",
  },
  "model.large": {
    "pt-BR": "Máxima precisão, muito lento",
    en: "Maximum accuracy, very slow",
    es: "Maxima precision, muy lento",
  },
  "model.turbo": {
    "pt-BR": "Rápido com boa precisão",
    en: "Fast with good accuracy",
    es: "Rapido con buena precision",
  },

  // Loading
  "loading.audio": {
    "pt-BR": "Transformando em áudio...",
    en: "Converting to audio...",
    es: "Convirtiendo a audio...",
  },
  "loading.text": {
    "pt-BR": "Transcrevendo...",
    en: "Transcribing...",
    es: "Transcribiendo...",
  },
  "loading.preparing": {
    "pt-BR": "Preparando...",
    en: "Preparing...",
    es: "Preparando...",
  },
  "loading.hint": {
    "pt-BR": "Isso pode levar alguns minutos",
    en: "This may take a few minutes",
    es: "Esto puede tardar unos minutos",
  },
  "loading.elapsed": {
    "pt-BR": "Tempo decorrido",
    en: "Elapsed time",
    es: "Tiempo transcurrido",
  },
  "loading.detectedLang": {
    "pt-BR": "Idioma detectado",
    en: "Detected language",
    es: "Idioma detectado",
  },

  // Result
  "result.title": {
    "pt-BR": "Transcrição",
    en: "Transcription",
    es: "Transcripcion",
  },
  "result.copy": {
    "pt-BR": "Copiar",
    en: "Copy",
    es: "Copiar",
  },
  "result.copied": {
    "pt-BR": "Copiado!",
    en: "Copied!",
    es: "Copiado!",
  },
  "result.new": {
    "pt-BR": "Nova transcrição",
    en: "New transcription",
    es: "Nueva transcripcion",
  },

  // Error
  "error.tryAgain": {
    "pt-BR": "Tentar novamente",
    en: "Try again",
    es: "Intentar de nuevo",
  },
  "error.copy": {
    "pt-BR": "Copiar detalhes",
    en: "Copy details",
    es: "Copiar detalles",
  },
  "error.copied": {
    "pt-BR": "Detalhes copiados!",
    en: "Details copied!",
    es: "Detalles copiados!",
  },

  // Upload - duration
  "upload.duration": {
    "pt-BR": "Duração",
    en: "Duration",
    es: "Duración",
  },

  // Cancel
  "loading.cancel": {
    "pt-BR": "Cancelar",
    en: "Cancel",
    es: "Cancelar",
  },
  "result.partial": {
    "pt-BR": "Transcrição parcial",
    en: "Partial transcription",
    es: "Transcripcion parcial",
  },

  // Settings
  "settings.title": {
    "pt-BR": "Configurações",
    en: "Settings",
    es: "Configuracion",
  },
  "settings.language": {
    "pt-BR": "Idioma",
    en: "Language",
    es: "Idioma",
  },
  "settings.deps": {
    "pt-BR": "Dependências",
    en: "Dependencies",
    es: "Dependencias",
  },
  "settings.start": {
    "pt-BR": "Começar",
    en: "Get started",
    es: "Comenzar",
  },
  "settings.save": {
    "pt-BR": "Voltar ao app",
    en: "Back to app",
    es: "Volver a la app",
  },
  "settings.depsRequired": {
    "pt-BR": "Instale todas as dependências para continuar",
    en: "Install all dependencies to continue",
    es: "Instala todas las dependencias para continuar",
  },

  // Updates
  "updates.title": {
    "pt-BR": "Atualizações",
    en: "Updates",
    es: "Actualizaciones",
  },
  "updates.current": {
    "pt-BR": "Versão atual",
    en: "Current version",
    es: "Versión actual",
  },
  "updates.latest": {
    "pt-BR": "Última versão",
    en: "Latest version",
    es: "Última versión",
  },
  "updates.upToDate": {
    "pt-BR": "Você está na versão mais recente",
    en: "You are on the latest version",
    es: "Estás en la versión más reciente",
  },
  "updates.available": {
    "pt-BR": "Nova versão disponível",
    en: "New version available",
    es: "Nueva versión disponible",
  },
  "updates.check": {
    "pt-BR": "Verificar atualizações",
    en: "Check for updates",
    es: "Buscar actualizaciones",
  },
  "updates.checking": {
    "pt-BR": "Verificando...",
    en: "Checking...",
    es: "Buscando...",
  },
  "updates.install": {
    "pt-BR": "Atualizar agora",
    en: "Update now",
    es: "Actualizar ahora",
  },
  "updates.downloading": {
    "pt-BR": "Baixando...",
    en: "Downloading...",
    es: "Descargando...",
  },
  "updates.installing": {
    "pt-BR": "Instalando...",
    en: "Installing...",
    es: "Instalando...",
  },
  "updates.error": {
    "pt-BR": "Falha ao verificar/atualizar",
    en: "Failed to check/update",
    es: "Error al buscar/actualizar",
  },
  "updates.restartHint": {
    "pt-BR": "O app será reiniciado após a atualização",
    en: "The app will restart after updating",
    es: "La app se reiniciará después de actualizar",
  },

  // History
  "history.title": {
    "pt-BR": "Histórico",
    en: "History",
    es: "Historial",
  },
  "history.back": {
    "pt-BR": "Voltar",
    en: "Back",
    es: "Volver",
  },
  "history.empty": {
    "pt-BR": "Nenhuma transcrição ainda",
    en: "No transcriptions yet",
    es: "Ninguna transcripción aún",
  },
  "history.delete": {
    "pt-BR": "Excluir",
    en: "Delete",
    es: "Eliminar",
  },
  "history.button": {
    "pt-BR": "Histórico",
    en: "History",
    es: "Historial",
  },
} as const;

type TranslationKey = keyof typeof translations;

export function t(key: TranslationKey, lang: Lang): string {
  return translations[key]?.[lang] ?? translations[key]?.["pt-BR"] ?? key;
}
