# 02 — Requisitos não funcionais

> Status: Aprovada · v1.0 · 2026-11-20
>
> Medições de desempenho em máquina de referência: Windows 11, CPU 4 núcleos ~2,5 GHz, 8 GB RAM, SSD.

## Desempenho (PERF)

| ID | Requisito | Meta | Como verificar |
|----|-----------|------|----------------|
| PERF-001 | Tempo entre o atalho global e o popup visível e com foco | p95 ≤ **150 ms** (popup pré-criado e oculto) | Teste E2E com timestamp + medição manual |
| PERF-002 | Captura de texto até persistência | p95 ≤ **50 ms** | Teste de desempenho ignorado por padrão (`crates/infrastructure/tests/perf.rs`, `--release --ignored`): p95 do caso de uso com SQLite em arquivo |
| PERF-003 | Captura de imagem 4K até persistência (PNG + miniatura) | p95 ≤ **800 ms**, fora da thread de UI e da thread do listener | Idem (imagem 4K de ruído, pior caso para o PNG) |
| PERF-004 | Busca em 5.000 itens | p95 ≤ **50 ms** | Idem (5.000 itens, termos variados) |
| PERF-005 | Memória em repouso (popup oculto) | ≤ **80 MB** working set (inclui WebView2) | Medição manual por release |
| PERF-006 | CPU em repouso | ~0% (sem polling; somente eventos) | Medição manual |
| PERF-007 | Inicialização até ícone na bandeja | ≤ **1,5 s** a frio | Medição manual |
| PERF-008 | Rolagem com 5.000 itens | 60 fps sem travamentos perceptíveis | `[manual]` + virtualização obrigatória (HIST-002) |

## Confiabilidade (REL)

| ID | Requisito |
|----|-----------|
| REL-001 | Escritas no SQLite em transação; banco em modo **WAL**; `synchronous=NORMAL`. |
| REL-002 | Gravação de imagem é atômica: escreve em arquivo temporário e renomeia; só então insere no banco. Falha no banco remove o arquivo. |
| REL-003 | Falha ao ler o clipboard (bloqueado por outro processo) é repetida até 5 vezes com backoff (10, 20, 40, 80, 160 ms) e depois descartada com log. |
| REL-004 | Nenhum `panic` em código de produção fora do composition root; erros tipados (`thiserror`) e propagados. `unwrap()`/`expect()` proibidos fora de testes (lint `clippy::unwrap_used`, `clippy::expect_used`). |
| REL-005 | Banco corrompido na inicialização: é renomeado para `history.corrupt-<timestamp>.db`, um novo banco é criado e o usuário é notificado. |
| REL-006 | Migrações de esquema são versionadas, idempotentes e testadas a partir de cada versão anterior publicada. |
| REL-007 | Logs rotativos em arquivo (`tracing` + `tracing-appender`), máx. 5 arquivos × 5 MB, nível padrão `info`. |

## Privacidade (PRIV)

| ID | Requisito |
|----|-----------|
| PRIV-001 | Todos os dados ficam **somente no dispositivo**. Nenhuma requisição de rede é feita pelo app (exceto abrir links no navegador a pedido do usuário). |
| PRIV-002 | Sem telemetria/analytics. |
| PRIV-003 | **Conteúdo do clipboard nunca é escrito em logs** — apenas metadados (tipo, tamanho, hash truncado). Teste automatizado verifica com um "conteúdo sentinela". |
| PRIV-004 | Respeito aos sinalizadores de exclusão (CAP-007) e à lista de apps excluídos (CAP-009). |
| PRIV-005 | Dados armazenados na pasta de dados do pacote (removida na desinstalação do MSIX). |
| PRIV-006 | Política de privacidade publicada (requisito da Store) descrevendo PRIV-001..005. |
| PRIV-007 | *(v1.x, avaliar)* Criptografia em repouso via DPAPI/SQLCipher. |

## Segurança (SEC)

| ID | Requisito |
|----|-----------|
| SEC-001 | Tauri com **CSP restritiva** (`default-src 'self'`; sem `unsafe-eval`; imagens apenas `self`/`asset:` do diretório de dados). |
| SEC-002 | **Capabilities do Tauri mínimas**: apenas os comandos próprios expostos; sem acesso genérico a FS/shell a partir do frontend. |
| SEC-003 | O frontend nunca recebe caminhos absolutos arbitrários; imagens são servidas por protocolo customizado (`clip://image/<id>`) que valida o ID. |
| SEC-004 | Todas as entradas vindas do frontend são validadas no backend (tipos de domínio com construtores validados). |
| SEC-005 | Consultas SQL somente parametrizadas. |
| SEC-006 | Dependências auditadas no CI (`cargo deny`, `cargo audit`, `npm audit`/`pnpm audit`). |
| SEC-007 | Capability MSIX limitada a `runFullTrust` (necessária para clipboard global, hotkeys e `SendInput`). |

## Usabilidade e acessibilidade (A11Y)

| ID | Requisito |
|----|-----------|
| A11Y-001 | 100% das ações do popup executáveis só com teclado. |
| A11Y-002 | Contraste mínimo **WCAG 2.1 AA** (4,5:1 texto, 3:1 componentes) nos dois temas — verificado por teste automatizado dos tokens de cor. |
| A11Y-003 | Compatível com leitor de tela (Narrador): papéis ARIA (`listbox`/`option`), rótulos em botões só com ícone, anúncio do item focado. |
| A11Y-004 | Respeita "Mostrar animações" do Windows / `prefers-reduced-motion`. |
| A11Y-005 | Respeita escala de texto do Windows e DPI até 300%; layout sem cortes. |
| A11Y-006 | Suporte a temas de alto contraste (`forced-colors`). |

## Manutenibilidade (MNT)

| ID | Requisito |
|----|-----------|
| MNT-001 | Arquitetura em camadas conforme [03](03-arquitetura.md); `domain` e `application` sem dependência de Windows, Tauri ou SQLite (verificado pela árvore de dependências do Cargo). |
| MNT-002 | `cargo fmt`, `cargo clippy -- -D warnings` (com `clippy::pedantic` seletivo) e ESLint/Prettier obrigatórios no CI. |
| MNT-003 | Cobertura mínima conforme [08](08-estrategia-de-testes.md). |
| MNT-004 | Funções públicas de `domain` e `application` documentadas (`#![warn(missing_docs)]`). |
| MNT-005 | Padrões de projeto apenas os catalogados em [04](04-padroes-de-projeto.md). |

## Compatibilidade (CMP)

| ID | Requisito |
|----|-----------|
| CMP-001 | Windows 11 (alvo principal) e Windows 10 22H2 (build 19045). `MinVersion` do manifesto: `10.0.19041.0`. |
| CMP-002 | Arquiteturas: **x64** e **arm64**. |
| CMP-003 | Dependência do WebView2 Evergreen Runtime (presente por padrão no Win11 e no Win10 atualizado). Na ausência, o app exibe instrução de instalação. |
| CMP-004 | Multi-monitor com DPIs diferentes (Per-Monitor V2). |
