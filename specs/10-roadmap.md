# 10 — Roadmap

> Status: Aprovada · v1.0 · 2026-11-20
>
> Cada marco termina com algo executável e testado. Um marco só começa com as specs dos seus requisitos `Aprovadas`.

| Marco | Objetivo | Requisitos | Critério de conclusão |
|-------|----------|-----------|-----------------------|
| **M0 — Fundações** | Workspace Cargo, crates vazias com a regra de dependência, app Tauri "hello tray", Svelte + Vitest, CI completo (lint, testes, cobertura, audit, trace-reqs) | MNT-001..002 | CI verde; `cargo tree` prova que `domain`/`application` não dependem de infra |
| ↳ *status M0* | ✅ Concluído localmente (2026-11-20). Falta o CI verde no GitHub, que depende do primeiro push. | | |
| **M1 — Domínio** | Value objects, entidades, `RetentionPolicy`, `Hotkey`, pipeline de filtros com fakes (TDD) | CAP-003..008, CAP-010, CAP-012, RET-001..003, HK-003, SNP-001 (regras) | Cobertura `domain` ≥ 95%, `application` ≥ 90%; property tests |
| ↳ *status M1* | ✅ Concluído (2026-11-20): domínio com 100% de cobertura de linhas, aplicação com 98%; property tests de `Hotkey` e `RetentionPolicy`. | | |
| **M2 — Persistência** | SQLite + migrações, repositórios, `ImageStore`, reconciliação, benchmarks | HIST-001..007 (backend), RET-004..005, REL-001..006, PERF-002..004 | Contract tests verdes p/ fakes e reais; benchmarks dentro das metas |
| ↳ *status M2* | ✅ Concluído (2026-11-20): contratos verdes para fakes e adaptadores reais; p95 medido: texto 0,2 ms, imagem 4K 379 ms, busca 1,7 ms. REL-003 (retentativas de leitura do clipboard) e REL-007 (logs em arquivo) ficaram para o M3, junto do adaptador Win32. | | |
| **M3 — Captura real** | Adaptadores Win32 de clipboard (listener, leitura, escrita, flags), worker de captura | CAP-001..012, PRIV-003..004 | Copiar em qualquer app → item no banco; teste sentinela de log |
| ↳ *status M3* | ✅ Concluído (2026-11-20): listener, leitura/escrita Win32 e worker; texto copiado em outro app chega ao banco (teste ponta a ponta e verificação manual no app); teste sentinela de logs; logs em arquivo. Pausa pela bandeja (TRAY-003) fica no M4. | | |
| **M4 — Popup e colagem** | Tray, popup pré-criado, lista virtualizada, busca, filtros, colar com retorno de foco, temas | TRAY-*, HIST-*, PST-*, HK-005, THM-001..003 | E2E: abrir → buscar → colar no Notepad; PERF-001 |
| ↳ *status M4* | ✅ Concluído (2026-11-20): popup com busca, filtros, lista virtualizada, teclado completo, pré-visualização, desfazer exclusão, menu e banner de pausa; colagem com retorno de foco e fallbacks; bandeja com pausar/retomar e ícone por tema; Acrylic no Win11; instância única. **Pendências:** E2E da UI com WebdriverIO não implementado (a colagem é coberta por E2E em Rust e a UI por testes de componente); PERF-001 sem medição formal; itens "Configurações"/"Sobre" da bandeja entram no M6. | | |
| **M5 — Atalhos e snippets** | `RegisterHotKey`, gravador de atalho, CRUD de snippets, colagem por atalho, restauração do clipboard | HK-*, SNP-* | E2E: `Ctrl+Shift+V` cola "Teste de texto" no Notepad |
| ↳ *status M5* | ✅ Concluído (2026-11-20): `RegisterHotKey` numa thread própria; atalho de abertura configurável; CRUD de snippets com validação e conflitos (internos e com outros apps); colagem por atalho que espera soltar os modificadores; restauração do clipboard; aba de snippets no popup; janela de configurações com a seção "Atalhos" e gravador de atalho. Testes Win32 reais cobrem atalho → colagem → restauração. As demais seções da janela entram no M6. | | |
| **M6 — Configurações, i18n e identidade** | Janela de configurações completa, PT-BR/EN, StartupTask, símbolo da marca (3 propostas → escolha), geração de todos os assets | CFG-*, I18N-*, THM-004, A11Y-*, 07 §6 | Assets gerados por script; contraste AA testado; axe sem violações |
| **M7 — Empacotamento e Store** | Manifesto, build MSIX x64/arm64, WACK, listagem, política de privacidade, submissão | 09, CMP-* | Pacote aprovado na certificação da Store |
| **v2** | Preservar formatação HTML/RTF como opção configurável (ligar/desligar) | — | Spec própria |
| **v1.x** | Arquivos (`CF_HDROP`), snippets com variáveis, criptografia em repouso, import/export, cor de destaque | — | Specs próprias |

## Decisões de produto

Revisadas e **confirmadas** em 2026-11-20 (antes eram "pontos em aberto"):

| # | Tema | Decisão | Alternativas descartadas |
|---|------|---------|--------------------------|
| 1 | Atalho padrão de abertura | `Ctrl + Win + V` | `Ctrl + Shift + Space`, `Alt + V` |
| 2 | Retenção padrão | 30 dias / 1.000 itens | outros valores |
| 3 | Restaurar clipboard após colar snippet | Ligado | Desligado |
| 4 | Snippet colado entra no histórico? | Não | Sim |
| 5 | Formatação (Word/HTML) | Salva só texto puro na v1 (para v2, será configurável, ou seja usuário pode optar por ativar ou desativar preservar html/rtf) | Preservar HTML/RTF já na v1 (aumenta escopo de M3/M4) |
| 6 | Suporte a Windows 10 | Sim (22H2) | Somente Windows 11 (simplifica Mica/Acrylic e testes) |
| 7 | Modelo de negócio na Store | Grátis | Grátis · Pago · Grátis + compra no app |
| 8 | Framework do frontend | Svelte 5 + TS (ADR-0006) | Leptos (UI também em Rust/WASM), SolidJS, React |
| 9 | Dados sensíveis | Respeitar flags de exclusão; sem criptografia na v1 | Criptografar o banco já na v1 |
