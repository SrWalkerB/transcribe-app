# Preferências de transcrição e abertura direta

## Objetivo

Eliminar a necessidade de selecionar repetidamente as opções de transcrição e
evitar que usuários já configurados caiam na tela de Configurações em toda
abertura do aplicativo.

## Escopo

### Preferências de transcrição

O frontend armazenará uma única chave versionada no `localStorage`:

```ts
{
  version: 1,
  model: "tiny" | "base" | "small" | "medium" | "large" | "turbo",
  device: "auto" | "gpu" | "cpu",
  threads: number
}
```

As preferências serão persistidas imediatamente quando o usuário escolher um
modelo, dispositivo ou quantidade de threads. Na próxima abertura, o seletor
começará com essas mesmas escolhas. Dados ausentes, JSON inválido, valores fora
da lista permitida ou threads fora do intervalo aceito serão descartados e os
padrões atuais serão usados (`base`, `auto`, `4`).

### Fluxo de início

O aplicativo iniciará em um estado breve de verificação, sem exibir a tela de
Configurações. Ele chamará o comando existente `check_dependencies` e avaliará
o resultado para a plataforma em uso:

- Sem o marcador local de configuração concluída: exibe Configurações.
- Marcador presente e todas as dependências disponíveis: abre diretamente o
  envio de vídeo.
- Falha na checagem ou dependência ausente: exibe Configurações, onde o usuário
  pode corrigir o problema.

Ao usar o botão “Get started” com as dependências válidas, a tela gravará o
marcador de configuração concluída. Usuários já existentes verão Configurações
uma vez após a atualização para criar esse marcador; as aberturas seguintes
serão diretas.

A engrenagem continua abrindo Configurações sob demanda. Erros de uma
transcrição não redirecionam a essa tela: seguem no estado de erro existente,
pois não implicam necessariamente que o ambiente esteja mal configurado.

## Implementação

- Extrair a leitura, validação e gravação das preferências para um módulo puro
  do frontend, compartilhado pelo `VideoUploader`.
- Alterar o estado inicial de `App` para a checagem e decidir entre `idle` e
  `settings` após a resposta do backend.
- Centralizar a avaliação de dependências por plataforma para que a decisão de
  início use o mesmo critério da tela de Configurações.
- Manter a configuração de idioma independente, como já ocorre hoje.
- Atualizar a versão Tauri de `0.1.18` para `0.1.20`; a tag `v0.1.20` dispara o
  workflow de release para Windows e Linux.

## Testes e validação

- Testar o módulo puro para restauração de dados válidos e fallback para dados
  ausentes, malformados ou inválidos.
- Testar a decisão de início para primeira execução, dependências saudáveis e
  dependências ausentes.
- Executar a verificação de tipos/build do frontend, `cargo check` e a checagem
  de diff antes da integração e da tag de release.

## Fora de escopo

- Sincronização das preferências entre dispositivos.
- Alterar o mecanismo de instalação de dependências.
- Redirecionar erros de arquivo ou de transcrição para Configurações.
