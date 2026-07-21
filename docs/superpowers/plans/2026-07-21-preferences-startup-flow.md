# Preferências de transcrição e abertura direta Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restaurar modelo, dispositivo e threads escolhidos e abrir diretamente o envio de vídeo depois de uma configuração válida.

**Architecture:** Um módulo puro controla as preferências e a decisão de abertura, permitindo testes sem Tauri ou React. `VideoUploader` consome e persiste as opções; `App` consulta `check_dependencies` uma vez na abertura e usa o marcador de configuração concluída para decidir entre `idle` e `settings`.

**Tech Stack:** React 19, TypeScript 5, Tauri 2, Vitest.

---

## Estrutura de arquivos

- `src/transcriptionPreferences.ts`: tipos, defaults, leitura/validação e persistência das opções.
- `src/startup.ts`: tipos de dependência e decisão pura da tela inicial.
- `src/transcriptionPreferences.test.ts` e `src/startup.test.ts`: regressões sem Tauri/React.
- `src/components/VideoUploader.tsx`: usa e grava a preferência única.
- `src/App.tsx`: faz a checagem de abertura e grava o marcador ao concluir setup.
- `src/components/SettingsPage.tsx`: reutiliza o critério de dependências compartilhado.
- `package.json` e `pnpm-lock.yaml`: disponibilizam Vitest.
- `src-tauri/tauri.conf.json`: atualiza a versão da release.

### Task 1: Infraestrutura de teste e preferências

**Files:**

- Modify: `package.json`
- Modify: `pnpm-lock.yaml`
- Create: `src/transcriptionPreferences.ts`
- Create: `src/transcriptionPreferences.test.ts`

- [ ] **Step 1: Escrever testes que falham para restauração e fallback**

```ts
import { describe, expect, it } from "vitest";
import { DEFAULT_TRANSCRIPTION_PREFERENCES, loadTranscriptionPreferences } from "./transcriptionPreferences";

describe("loadTranscriptionPreferences", () => {
  it("restaura preferência válida", () => {
    const storage = { getItem: () => JSON.stringify({ version: 1, model: "large", device: "gpu", threads: 12 }) };
    expect(loadTranscriptionPreferences(storage)).toEqual({ version: 1, model: "large", device: "gpu", threads: 12 });
  });

  it("usa padrões para dados inválidos", () => {
    expect(loadTranscriptionPreferences({ getItem: () => "{inválido" })).toEqual(DEFAULT_TRANSCRIPTION_PREFERENCES);
  });

  it("usa padrões para threads fora do intervalo", () => {
    const storage = { getItem: () => JSON.stringify({ version: 1, model: "base", device: "cpu", threads: 33 }) };
    expect(loadTranscriptionPreferences(storage)).toEqual(DEFAULT_TRANSCRIPTION_PREFERENCES);
  });
});
```

- [ ] **Step 2: Instalar o executor e confirmar a falha**

Adicionar `"test": "vitest run"` aos scripts e `"vitest": "^3.2.4"` às dependências de desenvolvimento.

Run: `pnpm install && pnpm test -- src/transcriptionPreferences.test.ts`

Expected: FAIL com módulo `./transcriptionPreferences` ausente.

- [ ] **Step 3: Implementar o módulo mínimo**

```ts
export const TRANSCRIPTION_PREFERENCES_KEY = "transcribe-options";
export const DEFAULT_TRANSCRIPTION_PREFERENCES = { version: 1 as const, model: "base", device: "auto", threads: 4 };
const MODELS = new Set(["tiny", "base", "small", "medium", "large", "turbo"]);
const DEVICES = new Set(["auto", "gpu", "cpu"]);

export function loadTranscriptionPreferences(storage: Pick<Storage, "getItem">) {
  try {
    const value: unknown = JSON.parse(storage.getItem(TRANSCRIPTION_PREFERENCES_KEY) ?? "null");
    if (typeof value === "object" && value !== null) {
      const data = value as Record<string, unknown>;
      if (data.version === 1 && typeof data.model === "string" && MODELS.has(data.model) &&
        typeof data.device === "string" && DEVICES.has(data.device) &&
        Number.isInteger(data.threads) && (data.threads as number) >= 1 && (data.threads as number) <= 32) {
        return { version: 1 as const, model: data.model, device: data.device, threads: data.threads as number };
      }
    }
  } catch {}
  return DEFAULT_TRANSCRIPTION_PREFERENCES;
}
```

Exportar também `saveTranscriptionPreferences(storage, { model, device, threads })`, que grava JSON com `version: 1`.

- [ ] **Step 4: Confirmar verde**

Run: `pnpm test -- src/transcriptionPreferences.test.ts && pnpm build`

Expected: teste passa e build termina com código 0.

- [ ] **Step 5: Commit**

```bash
git add package.json pnpm-lock.yaml src/transcriptionPreferences.ts src/transcriptionPreferences.test.ts
git commit -m "feat: persist transcription preferences"
```

### Task 2: Aplicar as preferências ao seletor

**Files:**

- Modify: `src/components/VideoUploader.tsx:1-70,260-345`
- Test: `src/transcriptionPreferences.test.ts`

- [ ] **Step 1: Inicializar o seletor pelas preferências**

Substituir os três `useState` por:

```ts
const [preferences, setPreferences] = useState(() => loadTranscriptionPreferences(localStorage));
const { model, device, threads } = preferences;

function updatePreferences(next: Omit<TranscriptionPreferences, "version">) {
  const saved = { version: 1 as const, ...next };
  setPreferences(saved);
  saveTranscriptionPreferences(localStorage, saved);
}
```

Usar `updatePreferences` em todos os controles de modelo, dispositivo e threads. Manter a chamada existente `onPathSelected(selected.path, model, threads, device)`.

- [ ] **Step 2: Confirmar regressões e build**

Run: `pnpm test -- src/transcriptionPreferences.test.ts && pnpm build`

Expected: testes passam e build termina com código 0.

- [ ] **Step 3: Commit**

```bash
git add src/components/VideoUploader.tsx
git commit -m "feat: restore transcription selections"
```

### Task 3: Decidir a tela inicial pelas dependências

**Files:**

- Create: `src/startup.ts`
- Create: `src/startup.test.ts`
- Modify: `src/App.tsx:1-40,169-214,341-520`
- Modify: `src/components/SettingsPage.tsx:1-35,146-160,238-242`

- [ ] **Step 1: Escrever testes que falham para a decisão de início**

```ts
import { describe, expect, it } from "vitest";
import { decideStartupState } from "./startup";

describe("decideStartupState", () => {
  const ready = { ffmpeg: true, python: false, faster_whisper: false, whisper_cli: true, model: true };
  it("abre envio depois do setup válido", () => expect(decideStartupState(true, "windows", ready)).toBe("idle"));
  it("abre configurações na primeira execução", () => expect(decideStartupState(false, "windows", ready)).toBe("settings"));
  it("abre configurações se falta dependência", () => expect(decideStartupState(true, "windows", { ...ready, model: false })).toBe("settings"));
});
```

- [ ] **Step 2: Confirmar falha**

Run: `pnpm test -- src/startup.test.ts`

Expected: FAIL com módulo `./startup` ausente.

- [ ] **Step 3: Implementar decisão pura e reutilizar o critério**

Em `src/startup.ts`, exportar:

```ts
export const SETUP_COMPLETE_KEY = "transcribe-setup-complete";
export function dependenciesReady(platform: Platform, deps: DependencyStatus | null) {
  return platform === "windows"
    ? Boolean(deps?.ffmpeg && deps?.whisper_cli && deps?.model)
    : Boolean(deps?.ffmpeg && deps?.python && deps?.faster_whisper);
}
export function decideStartupState(setupComplete: boolean, platform: Platform, deps: DependencyStatus | null) {
  return setupComplete && dependenciesReady(platform, deps) ? "idle" : "settings";
}
```

Trocar a expressão `allOk` de `SettingsPage` por `dependenciesReady(platform, deps)`.

- [ ] **Step 4: Ligar o App à decisão**

Iniciar `AppState` com `"checking"`. No efeito de montagem, chamar `get_platform` e `invoke<DependencyStatus>("check_dependencies")`, ler `localStorage.getItem(SETUP_COMPLETE_KEY) === "true"`, e aplicar `decideStartupState`; se qualquer chamada falhar, definir `settings`. Enquanto `checking`, exibir somente um indicador de carregamento. No callback que recebe a confirmação válida de `SettingsPage`, gravar o marcador antes de retornar a `idle`.

- [ ] **Step 5: Confirmar verde**

Run: `pnpm test && pnpm build`

Expected: todas as suítes passam e build termina com código 0.

- [ ] **Step 6: Commit**

```bash
git add src/startup.ts src/startup.test.ts src/App.tsx src/components/SettingsPage.tsx
git commit -m "feat: open directly after setup"
```

### Task 4: Versão, integração e release

**Files:**

- Modify: `src-tauri/tauri.conf.json:4`
- Modify: `docs/superpowers/specs/2026-07-21-preferences-startup-design.md`

- [ ] **Step 1: Atualizar versão**

Trocar `"version": "0.1.18"` por `"version": "0.1.20"`.

- [ ] **Step 2: Rodar validação completa**

Run: `pnpm test && pnpm build && cargo check`

Expected: os três comandos terminam com código 0.

- [ ] **Step 3: Conferir as mudanças antes de integrar**

Run: `git diff main...HEAD --check && git status --short`

Expected: sem erro de whitespace e apenas mudanças desta feature e da branch atual.

- [ ] **Step 4: Commit, merge e tag de release**

```bash
git add src-tauri/tauri.conf.json docs/superpowers/specs/2026-07-21-preferences-startup-design.md
git commit -m "chore: release v0.1.20"
git switch main
git merge --no-ff feat/windows-amd-gpu-whispercpp
git push origin main
git tag v0.1.20
git push origin v0.1.20
```

- [ ] **Step 5: Acompanhar publicação**

Run: `gh run list --workflow release.yml --limit 1` e `gh run watch <run-id> --exit-status`

Expected: jobs Windows e Linux com sucesso e release `v0.1.20` publicada.
