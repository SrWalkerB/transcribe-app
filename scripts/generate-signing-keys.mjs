#!/usr/bin/env node
/**
 * Gera o par de chaves de assinatura usado pelo updater do Tauri.
 *
 * Uso:
 *   node scripts/generate-signing-keys.mjs [--password <senha>] [--force]
 *
 * O que ele faz:
 *   1. Roda `tauri signer generate` e salva o par em .tauri/transcribe-app.key(.pub)
 *   2. Lê a chave pública e atualiza src-tauri/tauri.conf.json (plugins.updater.pubkey)
 *   3. Imprime as variáveis que você precisa colocar nos secrets do GitHub
 *
 * Secrets necessários no GitHub Actions:
 *   - TAURI_SIGNING_PRIVATE_KEY          → conteúdo de .tauri/transcribe-app.key
 *   - TAURI_SIGNING_PRIVATE_KEY_PASSWORD → a senha usada aqui
 *
 * A chave PÚBLICA vai commitada no tauri.conf.json — ela não é secreta.
 * A chave PRIVADA (.tauri/transcribe-app.key) NÃO deve ser commitada.
 */

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..");

const args = process.argv.slice(2);
const force = args.includes("--force");
const pwdIdx = args.indexOf("--password");
const password = pwdIdx >= 0 ? args[pwdIdx + 1] : "";

const keyDir = join(repoRoot, ".tauri");
const keyPath = join(keyDir, "transcribe-app.key");
const pubKeyPath = `${keyPath}.pub`;
const tauriConfPath = join(repoRoot, "src-tauri", "tauri.conf.json");

if (!existsSync(keyDir)) {
  mkdirSync(keyDir, { recursive: true });
}

if (existsSync(keyPath) && !force) {
  console.error(
    `❌ Já existe uma chave em ${keyPath}.\n` +
      `   Se quiser sobrescrever, rode de novo com --force (CUIDADO: a pubkey antiga para de funcionar para clientes já instalados).`
  );
  process.exit(1);
}

console.log("🔐 Gerando par de chaves com `tauri signer generate`...");

const npxCmd = process.platform === "win32" ? "npx.cmd" : "npx";
const genArgs = [
  "--yes",
  "tauri",
  "signer",
  "generate",
  "-w",
  keyPath,
  "-f",
];
if (password) {
  genArgs.push("-p", password);
}

const gen = spawnSync(npxCmd, genArgs, {
  cwd: repoRoot,
  stdio: "inherit",
});

if (gen.status !== 0) {
  console.error("❌ Falha ao gerar o par de chaves.");
  process.exit(gen.status ?? 1);
}

if (!existsSync(pubKeyPath)) {
  console.error(`❌ Não encontrei a chave pública esperada em ${pubKeyPath}.`);
  process.exit(1);
}

const rawPub = readFileSync(pubKeyPath, "utf8").replace(/\r/g, "");
const pubkeyLine = rawPub
  .split("\n")
  .map((l) => l.trim())
  .find((l) => /^[A-Za-z0-9+/=]+$/.test(l) && l.length > 40);

if (!pubkeyLine) {
  console.error(
    `❌ Conteúdo inesperado em ${pubKeyPath}. Esperava uma linha base64 com a pubkey.`
  );
  process.exit(1);
}

const conf = JSON.parse(readFileSync(tauriConfPath, "utf8"));
conf.plugins ??= {};
conf.plugins.updater ??= {};
conf.plugins.updater.pubkey = pubkeyLine;
writeFileSync(tauriConfPath, JSON.stringify(conf, null, 2) + "\n");

console.log("\n✅ Par de chaves gerado com sucesso.\n");
console.log(`   Privada: ${keyPath}`);
console.log(`   Pública: ${pubKeyPath}`);
console.log(
  `   tauri.conf.json atualizado com a nova pubkey (plugins.updater.pubkey).\n`
);

console.log("📋 Secrets para configurar no GitHub (Settings → Secrets → Actions):");
console.log("   1. TAURI_SIGNING_PRIVATE_KEY");
console.log(`      → copie TODO o conteúdo de ${keyPath}`);
console.log("   2. TAURI_SIGNING_PRIVATE_KEY_PASSWORD");
console.log("      → a senha que você usou ao gerar a chave\n");

console.log("⚠️  NÃO commite a chave privada. Confirme que .tauri/ está no .gitignore.");
