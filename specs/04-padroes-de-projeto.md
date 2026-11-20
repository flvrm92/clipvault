# 04 — Padrões de projeto

> Status: Aprovada · v1.0 · 2026-11-20
>
> **Regra:** um padrão só pode ser usado no código se estiver catalogado aqui com status `Aprovado`. Para introduzir um novo: adicionar a ficha abaixo (problema, solução, onde, consequências), revisar, e só então implementar. Código que usa o padrão referencia a ficha em comentário de módulo (ex.: `//! Pattern: Chain of Responsibility — specs/04 §P-05`).

## Princípios gerais

- **SOLID** aplicado a Rust:
  - **S** — um caso de uso por struct; adaptadores com uma responsabilidade cada.
  - **O** — novos filtros de captura / tipos de conteúdo entram sem alterar os existentes.
  - **L** — fakes e adaptadores reais cumprem o mesmo contrato, verificado por *contract tests* compartilhados ([08](08-estrategia-de-testes.md)).
  - **I** — portas pequenas e específicas (`ClipboardReader` ≠ `ClipboardWriter`).
  - **D** — casos de uso dependem de traits, nunca de tipos concretos de infraestrutura.
- **Clean Code:** nomes do domínio (glossário de [00](00-visao-produto.md)), funções curtas, sem flags booleanas em parâmetros públicos (usar enums), sem comentários que repetem o código.
- **Composição sobre herança**, imutabilidade por padrão, erros como valores.

## Catálogo

| ID | Padrão | Status |
|----|--------|--------|
| P-01 | Ports & Adapters (Hexagonal) | Aprovado |
| P-02 | Repository | Aprovado |
| P-03 | Injeção de dependência via construtor + Composition Root | Aprovado |
| P-04 | Command (casos de uso como comandos) | Aprovado |
| P-05 | Chain of Responsibility (pipeline de captura) | Aprovado |
| P-06 | Strategy (retenção e tratamento de formatos) | Aprovado |
| P-07 | Observer / Publish-Subscribe (eventos da aplicação) | Aprovado |
| P-08 | Value Object + Smart Constructor | Aprovado |
| P-09 | Data Transfer Object + Mapper | Aprovado |
| P-10 | Test Data Builder | Aprovado (somente testes) |
| P-11 | Fake / Test Double | Aprovado (somente testes) |
| P-12 | RAII Guard (recursos Win32) | Aprovado |
| P-13 | Active Object (threads com loop de mensagens) | Aprovado |

---

### P-01 · Ports & Adapters (Hexagonal)
- **Problema:** regras de negócio acopladas a Win32, SQLite e Tauri são impossíveis de testar sem o sistema real.
- **Solução:** `application` define *portas* (traits); `infrastructure` fornece *adaptadores*. O núcleo só conhece as portas.
- **Onde:** todas as portas listadas em [03 §3.2](03-arquitetura.md#32-application).
- **Consequências:** + testes rápidos e determinísticos; + troca de tecnologia localizada. − mais tipos e indireção; mitigado mantendo portas pequenas.

### P-02 · Repository
- **Problema:** casos de uso não devem conhecer SQL nem o layout do banco.
- **Solução:** `HistoryRepository`, `SnippetRepository`, `SettingsRepository` expõem operações em termos de domínio (`find_by_hash`, `search(query, page)`, `touch(id, at)`).
- **Onde:** `application::ports`, implementação em `infrastructure::sqlite`, fake em memória em `test-support`.
- **Consequências:** + SQL isolado e testado por testes de integração. − risco de repositório "genérico demais": proibido método `query(sql)` exposto.

### P-03 · Injeção de dependência via construtor + Composition Root
- **Problema:** criar dependências dentro dos casos de uso impede testes e viola DIP.
- **Solução:** casos de uso recebem as portas no construtor (`Arc<dyn Port>` ou genéricos `<R: HistoryRepository>`). Somente `app` (composition root) instancia adaptadores concretos.
- **Regra de escolha:** `Arc<dyn Trait>` por padrão, pela simplicidade de composição. Usar genéricos só onde uma medição mostrar custo relevante de despacho dinâmico. A captura acontece no ritmo humano (algumas vezes por minuto), então não se qualifica.
- Quando há muitas dependências, agrupá-las numa struct nomeada (ex.: `CaptureDependencies`) em vez de um construtor posicional longo.
- **Consequências:** + sem framework de DI; + dependências explícitas. − construtores com vários parâmetros; aceitável (≤ 6), acima disso revisar responsabilidade.

### P-04 · Command
- **Problema:** padronizar a execução de ações vindas da UI, de atalhos e do tray.
- **Solução:** cada caso de uso é um objeto com `execute(input) -> Result<Output, AppError>`. Comandos Tauri, hotkeys e tray apenas constroem o input e chamam o mesmo caso de uso.
- **Onde:** `application::use_cases::*`.
- **Consequências:** + um único caminho por ação (ex.: colar pelo popup e pelo tray usam `PasteItem`). − nenhum relevante.

### P-05 · Chain of Responsibility
- **Problema:** a decisão de capturar envolve muitas regras independentes (pausa, privacidade, app excluído, vazio, tamanho, duplicado) que crescem com o tempo.
- **Solução:** trait `CaptureFilter { fn name(&self) -> &'static str; fn apply(&self, ctx: CaptureContext) -> Result<FilterOutcome, AppError> }` com `FilterOutcome::{Continue(ctx), Stop(CaptureDecision)}`. A cadeia é uma `Vec<Box<dyn CaptureFilter>>` montada na ordem definida em [03 §3.2](03-arquitetura.md#32-application).
- **Consequências:** + cada regra testada isoladamente; + OCP. − ordem importa: um teste fixa a ordem da cadeia montada.

### P-06 · Strategy
- **Problema:** variações de algoritmo que não devem gerar `if`s espalhados.
- **Solução / onde:**
  - `RetentionRule` (por idade, por quantidade) compostas em `RetentionPolicy`.
  - `ImageDecoder` por formato de clipboard (`PNG`, `DIBV5`, `DIB`) escolhido por prioridade.
- **Consequências:** + fácil adicionar formatos (ex.: `CF_HDROP` na v1.x). − não usar para variações de um único caso.

### P-07 · Observer / Publish-Subscribe
- **Problema:** a UI precisa refletir mudanças (novo item, configuração) sem polling, e o núcleo não pode conhecer Tauri.
- **Solução:** porta `EventPublisher::publish(AppEvent)`; o adaptador Tauri converte em `emit`. No frontend, `listen` alimenta stores.
- **Consequências:** + desacoplamento. − eventos devem ser idempotentes ("algo mudou, recarregue a página atual"), evitando sincronizar estado incremental frágil.

### P-08 · Value Object + Smart Constructor
- **Problema:** valores inválidos (atalho sem modificador, nome vazio, texto acima do limite) circulando pelo sistema.
- **Solução:** newtypes com construtor `try_new`/`parse` que valida; campos privados; `Display`/`FromStr` canônicos. Ex.: `Hotkey`, `SnippetName`, `ClipText`, `ContentHash`.
- **Consequências:** + "estados inválidos irrepresentáveis"; + validação testada uma vez (inclusive com property tests). − conversões explícitas nas bordas.

### P-09 · DTO + Mapper
- **Problema:** tipos de domínio não devem ser acoplados ao formato serializado para a UI.
- **Solução:** `app::dto::*` com `serde`; conversões `From<&ClipItem> for ClipItemDto`. Tipos TypeScript gerados com `specta`/`ts-rs` para manter contrato.
- **Consequências:** + contrato UI/backend explícito e testado com snapshot. − duplicação controlada.

### P-10 · Test Data Builder *(testes)*
- **Problema:** montar entidades em testes polui os cenários.
- **Solução:** `ClipItemBuilder::text("abc").pinned().used_at(t).build()` em `test-support`.

### P-11 · Fake / Test Double *(testes)*
- **Problema:** testar casos de uso sem SO.
- **Solução:** fakes escritos à mão e com estado (`InMemoryHistoryRepository`, `FakeClipboard`, `FixedClock`, `RecordingKeystrokeSender`), preferidos a mocks. `mockall` permitido apenas para verificar interações difíceis de observar por estado.
- **Regra:** cada fake de porta passa pelo mesmo *contract test* que o adaptador real (garante LSP).

### P-12 · RAII Guard
- **Problema:** recursos Win32 (clipboard aberto, `HGLOBAL`, hotkey registrada, listener) vazam ou ficam travados em caminhos de erro.
- **Solução:** structs com `Drop` (ex.: `OpenClipboardGuard` chama `CloseClipboard`; `HotkeyRegistration` chama `UnregisterHotKey`).
- **Consequências:** + segurança em caminhos de erro/panic. − atenção para guards não atravessarem threads (`!Send` quando aplicável).

### P-13 · Active Object
- **Problema:** APIs Win32 de mensagens (`RegisterHotKey`, listener de clipboard) exigem thread com loop de mensagens; chamadas vêm de outras threads.
- **Solução:** thread dedicada dona do recurso, recebendo comandos por canal e emitindo eventos por outro canal. A porta expõe uma API síncrona/assíncrona simples.
- **Onde:** `win32::hotkeys`, `win32::clipboard::listener`.
- **Consequências:** + thread-safety sem locks sobre handles Win32. − encerramento ordenado obrigatório (mensagem de shutdown + `join`), coberto por teste.

## Anti-padrões proibidos

- Singleton global mutável (`static mut`, `lazy_static` com estado de negócio).
- Service Locator.
- Lógica de negócio em comandos Tauri ou no frontend.
- `unwrap`/`expect` em produção (ver REL-004).
