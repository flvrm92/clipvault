# 08 — Estratégia de testes

> Status: Aprovada · v1.0 · 2026-11-20

## 1. Princípios

- **TDD obrigatório** para `domain` e `application`: vermelho → verde → refatorar. Nenhum código de produção sem um teste que falhou antes.
- **Testar comportamento, não implementação:** asserts sobre resultados e estado observável; evitar verificar chamadas internas.
- **Determinismo:** tempo via `Clock` (fake), sem `sleep` em testes unitários, sem dependência de ordem.
- **Rápido:** suíte unitária completa < 10 s.
- **Fakes > mocks** (ver P-11 em [04](04-padroes-de-projeto.md)).

## 2. Pirâmide

| Nível | Escopo | Ferramentas | Roda onde |
|-------|--------|-------------|-----------|
| **Unitário** | `domain`, `application` (com fakes), utilitários da UI | `cargo test`, `proptest`, `rstest`, Vitest | Qualquer SO, a cada commit |
| **Contrato** | Mesma suíte aplicada a fake e adaptador real de cada porta | Macros/funções genéricas em `test-support` | Qualquer SO (fakes) / Windows (adaptadores Win32) |
| **Integração** | `infrastructure::sqlite` (arquivo temporário), `fs_images` (`tempfile`), migrações | `cargo test`, `tempfile`, `insta` | Qualquer SO |
| **Integração Win32** | Clipboard real, `RegisterHotKey`, `SendInput` em janela de teste | `cargo test -- --ignored` + `serial_test` (clipboard é global) | Windows (CI `windows-latest`) |
| **Componente (UI)** | Componentes Svelte isolados, stores, api mockada | Vitest + `@testing-library/svelte` | Qualquer SO |
| **E2E** | App empacotado: abrir popup, buscar, colar no Notepad, snippet | `tauri-driver` + WebdriverIO | Windows |
| **Manual** | Itens marcados `[manual]`, apps elevados, multi-monitor/DPI | Roteiro em `docs/testing/manual-checklist.md` | Antes de cada release |

## 3. O que testar por camada (exemplos obrigatórios)

### domain
- `Hotkey::parse`: válidos, inválidos, normalização (`ctrl+shift+1` ≡ `Shift+Ctrl+1`), reservados.
  - **Property test:** `parse(display(h)) == h` para todo `Hotkey` gerado.
- `ClipText`: vazio/espaços rejeitados, limite de tamanho, `preview` (3 linhas, colapsa whitespace, reticências), `normalized_for_search` (acentos, caixa).
- `RetentionPolicy::select_expired`: idade, quantidade, fixados, combinações, limites exatos (`==` max_age).
  - **Property test:** nunca seleciona fixados; após remoção, `não_fixados ≤ max_items`.
- `ContentHash`: mesma imagem em representações diferentes → mesmo hash.

### application
- Cada filtro do pipeline isoladamente + teste da **ordem** da cadeia montada.
- `CaptureClipboard`: um teste por CA de CAP-001..012.
- `PasteItem`: sucesso; janela alvo indisponível (PST-005); promoção do item; marca de captura própria.
- `PasteSnippet`: espera de modificadores; restauração do clipboard ligada/desligada; nada registrado no histórico.
- `ChangeOpenHotkey`/`SaveSnippet`: conflitos internos, conflito com outro app (registrar fake retorna erro), rollback mantém o anterior.
- `PurgeExpired`: aciona remoção de arquivos de imagem.

### infrastructure
- Repositórios: CRUD, ordenação (fixados primeiro), paginação, busca sem acento, `ON CONFLICT` de hash, cascata de `image_assets`.
- Retenção SQL **vs.** função pura de domínio (oráculo) com casos gerados.
- Migrações: banco vazio → última; cada versão publicada → última.
- `ImageStore`: atomicidade (falha simulada antes do rename não deixa arquivo), miniatura com proporção, reconciliação.
- Win32 (ignored/serial): escrever texto e ler de volta; imagem DIB ↔ PNG; flag `ExcludeClipboardContentFromMonitorProcessing` detectada; sequence number muda; registro duplicado de hotkey retorna conflito.

### app
- DTOs: snapshot (`insta`) do JSON serializado — garante contrato com a UI.
- Tipos TS gerados (`specta`/`ts-rs`) iguais aos versionados (CI falha se divergirem).

### ui
- Navegação por teclado no popup (tabela de [06 §2.2](06-ux-ui.md#22-teclado-no-popup)).
- Gravador de atalho: estados e mensagens.
- i18n: chaves PT-BR == EN (I18N-004); nenhuma string literal em componentes (regra ESLint).
- Tokens: contraste AA em ambos os temas (A11Y-002).
- Acessibilidade: `axe-core` sem violações nas telas principais.

### Win32
- `crates/infrastructure/tests/win32_clipboard.rs`: testes `#[ignore]` contra o clipboard real, executados em série: `cargo test -p clipvault-infrastructure --test win32_clipboard -- --ignored --test-threads=1`. Uma sessão salva e restaura o texto do usuário. Cobrem leitura/escrita de texto e imagem, DIB, flags de privacidade, dono, listener e o fluxo ponta a ponta até o SQLite. Rodam no CI (`windows-latest`).
- `crates/infrastructure/tests/win32_paste.rs`: colagem ponta a ponta (clipboard + foco + `SendInput`) numa janela de edição real. Fica num **binário de teste próprio** porque o Windows só deixa trocar a janela em primeiro plano a um processo que já está em primeiro plano. Um processo recém-iniciado tem esse direito para a primeira janela, e o teste reproduz a situação do app real: o "popup" do processo está em foco e cola em outra janela.

### Desempenho
- `crates/infrastructure/tests/perf.rs`: testes `#[ignore]` que medem o p95 de PERF-002..004 com os adaptadores reais. Rodar em release na máquina de referência antes de cada release: `cargo test -p clipvault-infrastructure --release --test perf -- --ignored --nocapture`. Ficam fora do CI porque runners compartilhados não dão tempos estáveis.

### Privacidade
- **Teste sentinela (PRIV-003):** captura um texto `SENTINEL-<uuid>`, executa fluxos principais com log em nível `trace` e verifica que o sentinela não aparece em nenhum arquivo de log.

### Organização dos testes Rust

- Testes unitários de `domain` ficam em `src/<módulo>/tests.rs` (`#[cfg(test)] mod tests;`).
- Testes de `application` que usam fakes ficam em `crates/application/tests/`. Motivo: `test-support` depende de `application`; se os testes internos do crate usassem os fakes, o Cargo compilaria duas cópias dos tipos de `application`, incompatíveis entre si. Lógica pura sem fakes (ex.: `Debouncer`) pode ser testada dentro de `src`.
- Em `tests/*.rs` e em `test-support`, `unwrap`/`expect`/`panic` são liberados por atributo no topo do arquivo, porque é código de teste.

## 4. Rastreabilidade

- Nome do teste ou atributo contém o ID do requisito: `fn cap_004_duplicate_text_is_promoted_not_inserted()`; em Vitest: `it('[HIST-003] busca ignora acentos', …)`.
- Script `tools/trace-reqs` lista requisitos `M`/`S` de [01](01-requisitos-funcionais.md) sem teste correspondente. **CI falha** se algum `M` aprovado não tiver teste (exceto os marcados `[manual]`).

## 5. Metas de cobertura (linhas, `cargo llvm-cov`)

| Componente | Mínimo |
|------------|--------|
| `domain` | 95% |
| `application` | 90% |
| `infrastructure` (excluindo `win32`) | 85% |
| `infrastructure::win32` | 60% (complementado por testes Win32 ignored + manuais) |
| `ui` (Vitest) | 80% |
| **Total Rust** | 85% |

- **Mutation testing** (`cargo-mutants`) em `domain` e `application` semanalmente / antes de release; meta: ≥ 80% dos mutantes mortos.
- Cobertura é um piso, não o objetivo — revisões checam a qualidade dos asserts.

## 6. CI (GitHub Actions)

Implementado em [`.github/workflows/ci.yml`](../.github/workflows/ci.yml):

```
on: push (main) / pull_request
jobs:
  rust      (windows-latest): cargo fmt --check · cargo clippy --all-targets (-D warnings)
                              · cargo test --workspace · cargo xtask check-arch · cargo xtask trace-reqs
                              · [M3+] cargo test -- --ignored (Win32, serial)
  coverage  (windows-latest): por camada (domain 95 · application 90 · infra sem win32 85)
                              · win32 com seus testes reais (60) · total sem app/win32 (85)
  ui        (windows-latest): pnpm lint · pnpm check · pnpm coverage (limite 80%) · pnpm build
  audit     (ubuntu-latest) : cargo deny check · pnpm audit --prod
  [M6] assets              : tools/gen-assets --check
  [M4+] e2e (nightly/release): build MSIX de teste · tauri-driver + WebdriverIO
```

- O crate `app` (composition root) fica fora da cobertura de linhas: é validado por testes E2E.
- `cargo xtask check-arch` verifica a regra de dependência (MNT-001) sobre o grafo do `cargo metadata`, inclusive dependências transitivas proibidas.
- `cargo xtask trace-reqs` lê os requisitos de `01`/`02`, procura referências nos testes e aplica o escopo de [`trace-scope.toml`](trace-scope.toml). Esse escopo cresce a cada marco concluído. Também falha se um teste referenciar um ID inexistente.

## 7. Definição de pronto (por requisito)

- [ ] Spec `Aprovada` e CAs claros.
- [ ] Testes escritos antes e falhando; agora passando.
- [ ] Cobertura da camada mantida acima do mínimo.
- [ ] Clippy/fmt/lint limpos; sem `unwrap` em produção.
- [ ] Strings em PT-BR e EN.
- [ ] Padrões usados constam em [04](04-padroes-de-projeto.md).
- [ ] Docs/ADR atualizados se houve decisão nova.
