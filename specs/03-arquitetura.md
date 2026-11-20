# 03 — Arquitetura

> Status: Aprovada · v1.0 · 2026-11-20 · Decisões em [docs/adr](../docs/adr/)

## 1. Visão geral

**Clean Architecture / Hexagonal (Ports & Adapters)** sobre **Tauri 2**. Toda regra de negócio está em Rust, em crates puras e testáveis sem Windows. O frontend (WebView2) é **apenas apresentação**: renderiza estado e envia intenções ao backend.

```
┌───────────────────────────────────────────────────────────────────┐
│ ui/  (Svelte 5 + TypeScript, WebView2)                            │
│   views · componentes · stores · i18n · tokens de tema            │
└───────────────▲───────────────────────────────┬───────────────────┘
                │ eventos Tauri (push)          │ invoke (comandos)
┌───────────────┴───────────────────────────────▼───────────────────┐
│ app/  (crate binária: src-tauri)  — COMPOSITION ROOT              │
│   comandos Tauri finos · DTOs · tray · janelas · protocolo clip://│
└───────────────▲───────────────────────────────┬───────────────────┘
                │ chama casos de uso            │ injeta adaptadores
┌───────────────┴──────────────────┐  ┌─────────▼──────────────────┐
│ application/                     │  │ infrastructure/            │
│  casos de uso · PORTAS (traits)  │◄─┤  sqlite · fs_images ·      │
│  pipeline de captura · eventos   │  │  win32 (clipboard, input,  │
└───────────────▲──────────────────┘  │  hotkeys, foco, startup)   │
                │                     └─────────┬──────────────────┘
┌───────────────┴──────────────────┐            │
│ domain/                          │◄───────────┘
│  entidades · value objects ·     │
│  regras · erros de domínio       │
└──────────────────────────────────┘
```

**Regra de dependência:** as setas de dependência apontam para dentro. `domain` não depende de nada do projeto; `application` depende só de `domain`; `infrastructure` implementa portas de `application`; `app` conhece todos e faz a ligação.

## 2. Estrutura do repositório

```
clipvault/
├─ Cargo.toml                 # workspace
├─ crates/
│  ├─ domain/                 # clipvault-domain      (sem deps de IO)
│  ├─ application/            # clipvault-application (deps: domain, thiserror)
│  ├─ infrastructure/         # clipvault-infra       (rusqlite, image, windows)
│  └─ test-support/           # builders, fakes compartilhados (dev-dependency)
├─ app/                       # src-tauri: binário, tauri.conf.json, capabilities/
├─ ui/                        # frontend Svelte + Vite + Vitest
├─ packaging/
│  └─ msix/                   # AppxManifest.xml, assets gerados, scripts
├─ assets/
│  └─ brand/                  # SVGs mestres da identidade visual
├─ tools/                     # scripts: geração de assets, build MSIX
├─ docs/adr/                  # decisões de arquitetura
└─ specs/                     # esta especificação
```

## 3. Camadas em detalhe

### 3.1 `domain`

Tipos puros, imutáveis quando possível, com construtores validados ("parse, don't validate").

| Tipo | Descrição |
|------|-----------|
| `ClipItem` | Entidade: `id`, `content: ClipContent`, `hash: ContentHash`, `created_at`, `last_used_at`, `use_count`, `pinned`, `source_app: Option<AppName>` |
| `ClipContent` | `enum { Text(ClipText), Image(ImageMeta) }` |
| `ClipText` | Texto não vazio, ≤ limite; expõe `preview(max_lines)` e `normalized_for_search()` |
| `ImageMeta` | `width`, `height`, `byte_size`, `file_key` (identificador do arquivo, não caminho) |
| `ContentHash` | SHA-256 do conteúdo normalizado |
| `Snippet` | `id`, `name: SnippetName`, `body: SnippetBody`, `hotkey: Option<Hotkey>`, `enabled`, `order` |
| `Hotkey` | Value object: `modifiers: Modifiers` (bitflags) + `key: Key`; `parse("Ctrl+Shift+1")`, `Display` canônico, `is_reserved()` |
| `RetentionPolicy` | `max_age: Option<Duration>`, `max_items: Option<NonZeroU32>`; `select_expired(items, now) -> Vec<ClipId>` (função pura) |
| `Settings` | Agregado de configurações com padrões (`Default`) |
| `CaptureDecision` | `enum { Store(NewClip), Promote(ClipId), Ignore(IgnoreReason) }`. `NewClip` carrega o conteúdo capturado (`CapturedContent::{Text(ClipText), Image(ImagePixels)}`) com hash e origem; a imagem ainda não foi gravada em disco |
| `ImagePixels` | Imagem decodificada em RGBA 8 bits (validada: `len == w × h × 4`), como lida do clipboard |
| `Timestamp` | Instante UTC em milissegundos Unix (formato do banco) |
| `DomainError` | Erros de validação tipados |

### 3.2 `application`

**Portas (traits)** — definidas aqui, implementadas na infraestrutura:

| Porta | Responsabilidade |
|-------|------------------|
| `ClipboardReader` | Ler o snapshot atual (`ClipboardSnapshot`: formatos presentes, texto, imagem bruta, flags de privacidade, dono) |
| `ClipboardWriter` | Escrever texto/imagem; devolve o `sequence_number` resultante (para CAP-010) |
| `HistoryRepository` | CRUD/consulta de `ClipItem`, busca, paginação, `find_by_hash` |
| `SnippetRepository` | CRUD de `Snippet` |
| `SettingsRepository` | Ler/gravar `Settings` |
| `ImageStore` | Salvar/ler/remover PNG + miniatura por `file_key`; listar chaves (reconciliação) |
| `KeystrokeSender` | Enviar `Ctrl+V`; aguardar liberação de modificadores (SNP-004) |
| `FocusTracker` | Capturar e restaurar a janela alvo; posição do caret |
| `HotkeyRegistrar` | Registrar/desregistrar atalhos globais; reporta conflito |
| `StartupManager` | Habilitar/consultar início automático |
| `Clock` | `now()` — injetável para testes de retenção |
| `EventPublisher` | Publicar `AppEvent` (ex.: `HistoryChanged`) para a UI |

**Casos de uso** (um struct por caso de uso, método `execute`):

| Caso de uso | Requisitos |
|-------------|-----------|
| `CaptureClipboard` | CAP-001..012 |
| `ListHistory` / `SearchHistory` | HIST-001..004 |
| `TogglePin`, `DeleteItem`, `ClearHistory` | HIST-005..007 |
| `PasteItem` | PST-001..005, CAP-010 |
| `CopyItem` | HIST-011 |
| `PurgeExpired`, `ReconcileImages` | RET-001..005 |
| `SaveSnippet`, `DeleteSnippet`, `ListSnippets` | SNP-001..002, SNP-006 |
| `PasteSnippet` | SNP-003..005, SNP-007 |
| `UpdateSettings`, `GetSettings` | CFG, THM, I18N, RET |
| `ChangeOpenHotkey` | HK-001..004 |
| `ApplyHotkeyBindings` | sincroniza todos os atalhos com o sistema na inicialização/alteração |

**Pipeline de captura** — `CaptureClipboard::execute(snapshot)` recebe o `ClipboardSnapshot` já lido pelo worker (a leitura com retentativas, REL-003, fica no adaptador) e aplica uma cadeia de filtros (ver padrão *Chain of Responsibility* em [04](04-padroes-de-projeto.md)):

```
snapshot → PausedFilter → OwnWriteFilter → PrivacyFlagsFilter → ExcludedAppFilter
         → ContentSelector (texto > imagem) → EmptyFilter → SizeLimitFilter
         → Deduplicator (hash) → CaptureDecision
```

Componentes de apoio em `application::capture`:
- `OwnWriteRegistry`: números de sequência das escritas do próprio app (CAP-010), compartilhado entre os casos de uso que escrevem no clipboard e o `OwnWriteFilter`.
- `Debouncer`: lógica pura do debounce de 100 ms (CAP-012), dirigida por timestamps. O listener Win32 (M3) só agenda o timer.

### 3.3 `infrastructure`

| Módulo | Implementa | Tecnologia |
|--------|-----------|------------|
| `sqlite` | `HistoryRepository`, `SnippetRepository`, `SettingsRepository` | `rusqlite` (feature `bundled`), `rusqlite_migration` |
| `fs_images` | `ImageStore` | `image` (PNG encode/decode, resize para miniatura) |
| `win32::clipboard` | `ClipboardReader`, `ClipboardWriter`, listener | `windows` crate: `AddClipboardFormatListener` em janela *message-only*, `WM_CLIPBOARDUPDATE`, `GetClipboardSequenceNumber` |
| `win32::input` | `KeystrokeSender` | `SendInput`, `GetAsyncKeyState` |
| `win32::focus` | `FocusTracker` | `GetForegroundWindow`, `SetForegroundWindow` (+ `AllowSetForegroundWindow`/`AttachThreadInput` quando necessário), `GetGUIThreadInfo` |
| `win32::hotkeys` | `HotkeyRegistrar` | `RegisterHotKey`/`UnregisterHotKey` em thread dedicada com loop de mensagens |
| `win32::startup` | `StartupManager` | WinRT `Windows.ApplicationModel.StartupTask` |
| `system_clock` | `Clock` | `time`/`std` |

> Os atalhos globais são implementados diretamente com `RegisterHotKey` (e não com `tauri-plugin-global-shortcut`) para ter controle de erro por atalho (HK-004) e testabilidade via porta. Ver ADR-0005.

### 3.4 `app` (composition root)

- Cria adaptadores, injeta nos casos de uso e guarda tudo em `tauri::State<AppContainer>`.
- **Comandos Tauri** são finos: convertem DTO → tipos de domínio, chamam o caso de uso, convertem resultado → DTO. Sem regra de negócio.
- **Eventos** para a UI: `history://changed`, `settings://changed`, `theme://changed`, `capture://paused`.
- **Janelas:** `popup` (pré-criada, oculta, sem decoração, `skipTaskbar`, sempre no topo) e `settings` (criada sob demanda).
- **Tray:** `tauri::tray` com menu e troca de ícone (TRAY-002/003, THM-003).
- **Protocolo `clip://`:** serve imagens/miniaturas por ID validado (SEC-003).
- **Plugins:** `tauri-plugin-single-instance` (TRAY-004).

### 3.5 `ui`

- **Svelte 5 + TypeScript + Vite** (ADR-0006). Sem lógica de negócio: stores refletem estado vindo do backend.
- Camada `api/` única que encapsula `invoke`/`listen` (facilita mock em testes).
- Tokens de design em CSS custom properties (ver [07](07-identidade-visual.md)); temas via `data-theme`.
- i18n com arquivos JSON por idioma (`ui/src/i18n/pt-BR.json`, `en.json`).

## 4. Modelo de concorrência

| Thread / tarefa | Função |
|-----------------|--------|
| Main (Tauri/UI) | Event loop, janelas, tray |
| `clipboard-listener` | Janela message-only, recebe `WM_CLIPBOARDUPDATE`, aplica debounce (CAP-012) e envia `ClipboardChanged` por canal |
| `hotkeys` | Loop de mensagens com `RegisterHotKey`; recebe comandos (registrar/remover) por canal e emite `HotkeyPressed(id)` |
| `capture-worker` | Consome eventos, executa `CaptureClipboard` (inclui encode PNG); publica `HistoryChanged` |
| `maintenance` | Timer horário de `PurgeExpired` |

Comunicação por canais (`crossbeam-channel` ou `std::sync::mpsc`). O acesso ao SQLite usa uma conexão por thread com WAL (ou pool `r2d2_sqlite` pequeno).

## 5. Fluxos principais

### 5.1 Captura

```
App X copia → WM_CLIPBOARDUPDATE → listener (debounce 100ms)
  → capture-worker: ClipboardReader.snapshot()
  → pipeline de filtros → CaptureDecision
     Store(content): [imagem] ImageStore.save (tmp→rename) → HistoryRepository.insert
     Promote(id):    HistoryRepository.touch(id)
     Ignore(reason): log (sem conteúdo)
  → PurgeExpired (regra de quantidade)
  → EventPublisher.publish(HistoryChanged) → UI atualiza (HIST-009)
```

### 5.2 Abrir popup e colar

```
Atalho global → HotkeyPressed(OPEN)
  → FocusTracker.capture_target()        # guarda HWND alvo + caret
  → posiciona e mostra popup (pré-criado), foca busca
Usuário escolhe item → invoke paste_item(id)
  → PasteItem: ClipboardWriter.write(content) → registra seq. própria (CAP-010)
  → esconde popup → FocusTracker.restore_target()
     ok   → KeystrokeSender.send_paste()
     erro → notificação "Conteúdo copiado" (PST-005)
  → HistoryRepository.touch(id) → HistoryChanged
```

### 5.3 Snippet por atalho

```
Atalho do snippet → HotkeyPressed(SNIPPET(id))
  → PasteSnippet: snapshot anterior do clipboard (se SNP-005 ligado)
  → ClipboardWriter.write(body) (marcado como captura própria, não registra)
  → KeystrokeSender.wait_modifiers_released(1s) → send_paste()
  → após 500ms: ClipboardWriter.write(snapshot anterior) (também própria)
```

## 6. Tratamento de erros

- `domain`: `DomainError` (validação).
- `application`: `AppError` agrega erros de domínio e de portas (`#[from]`), sem detalhes de IO.
- `infrastructure`: erros próprios convertidos para os erros das portas.
- `app`: converte `AppError` em `CommandError { code, message_key }` serializável; a UI traduz `message_key` via i18n.

## 7. Configuração e caminhos

| Dado | Local (resolvido via Tauri `app_local_data_dir`; no MSIX é virtualizado para a pasta do pacote) |
|------|------|
| Banco | `<data>/history.db` |
| Imagens | `<data>/images/<aa>/<file_key>.png` e `<data>/images/<aa>/<file_key>.thumb.png` (`aa` = 2 primeiros caracteres da chave, para evitar diretórios enormes) |
| Logs | `<data>/logs/` |
