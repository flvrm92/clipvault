# ClipVault — Especificações

Este diretório é a **fonte da verdade** do produto. O projeto segue **SDD (Spec-Driven Development)**: nenhuma funcionalidade é implementada sem uma spec aprovada, e toda mudança de comportamento começa por uma alteração aqui.

## Índice

| # | Documento | Conteúdo |
|---|-----------|----------|
| 00 | [Visão do produto](00-visao-produto.md) | Problema, público, escopo, fora de escopo, glossário |
| 01 | [Requisitos funcionais](01-requisitos-funcionais.md) | Requisitos com IDs e critérios de aceite (Gherkin) |
| 02 | [Requisitos não funcionais](02-requisitos-nao-funcionais.md) | Desempenho, privacidade, segurança, acessibilidade, confiabilidade |
| 03 | [Arquitetura](03-arquitetura.md) | Camadas, crates, portas e adaptadores, fluxos principais |
| 04 | [Padrões de projeto](04-padroes-de-projeto.md) | Catálogo de padrões **aprovados** para uso no código |
| 05 | [Modelo de dados](05-modelo-de-dados.md) | Esquema SQLite, armazenamento de imagens, migrações, retenção |
| 06 | [UX / UI](06-ux-ui.md) | Telas, interações, teclado, temas, i18n |
| 07 | [Identidade visual](07-identidade-visual.md) | Marca, paleta, tipografia, ícones e lista de assets |
| 08 | [Estratégia de testes](08-estrategia-de-testes.md) | TDD, pirâmide de testes, ferramentas, metas de cobertura, CI |
| 09 | [Distribuição (Microsoft Store)](09-distribuicao-msix.md) | MSIX, manifesto, capabilities, startup task, submissão |
| 10 | [Roadmap](10-roadmap.md) | Marcos incrementais e critérios de conclusão |
| — | [ADRs](../docs/adr/) | Registro das decisões de arquitetura |

## Processo SDD adotado

1. **Especificar** — o requisito é escrito (ou alterado) em `01`/`02` com ID estável e critérios de aceite.
2. **Revisar** — a spec é revisada e muda de status `Rascunho` → `Aprovada`.
3. **Planejar** — decisões técnicas relevantes viram ADR em `docs/adr/`. Padrões de projeto novos entram em `04` **antes** de serem usados.
4. **Testar primeiro (TDD)** — cada critério de aceite vira ao menos um teste, que deve falhar antes da implementação.
5. **Implementar** — o mínimo para passar; depois refatorar.
6. **Rastrear** — testes referenciam o ID do requisito (ver [08](08-estrategia-de-testes.md#rastreabilidade)).

## Convenções

- **IDs de requisito:** `<ÁREA>-<NNN>` (ex.: `HIST-003`). IDs nunca são reutilizados; requisito removido fica marcado como `Removido`.
- **Prioridade (MoSCoW):** `M` (Must), `S` (Should), `C` (Could), `W` (Won't nesta versão).
- **Status do documento:** `Rascunho` · `Em revisão` · `Aprovada` · `Obsoleta`.
- **Idioma:** specs e docs em PT-BR; código, identificadores, commits e mensagens de log em inglês.

## Status atual

| Documento | Status |
|-----------|--------|
| 00–10 | **Aprovada** (v1.0 — 2026-11-20) |
