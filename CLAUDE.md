# ClipVault — instruções para agentes

## Processo (SDD + TDD)
- `specs/` é a fonte da verdade. Não implemente comportamento sem requisito em `specs/01` ou `specs/02`; mudança de comportamento começa alterando a spec.
- Só use padrões de projeto catalogados em `specs/04-padroes-de-projeto.md`. Padrão novo: documente a ficha lá **antes** de usar.
- TDD em `domain` e `application`: teste falhando primeiro. Nomes de teste incluem o ID do requisito (`cap_004_...`, `it('[HIST-003] ...')`).
- Decisão técnica relevante → novo ADR em `docs/adr/`.
- Toda mudança no app sobe a versão do `Cargo.toml` acima da última tag publicada e atualiza a seção `## [X.Y.Z] - Unreleased` do CHANGELOG (MNT-006, regras em `specs/09` §9).

## Arquitetura
- Regra de dependência: `domain` ← `application` ← `infrastructure` ← `app`. `domain`/`application` nunca dependem de Windows, Tauri ou SQLite.
- Comandos Tauri e frontend sem regra de negócio.
- Sem `unwrap`/`expect` fora de testes. Nunca logar conteúdo do clipboard.

## Verificação antes de concluir
`cargo fmt --all --check` · `cargo clippy --workspace --all-targets` · `cargo test --workspace` · `cargo xtask check-arch` · `cargo xtask trace-reqs` · `cargo xtask check-version` · `pnpm --dir ui lint && pnpm --dir ui check && pnpm --dir ui coverage`.
Ao concluir um marco, adicionar os requisitos cobertos em `specs/trace-scope.toml`.

## Idioma
- Specs/docs em PT-BR; código, identificadores, commits e logs em inglês. UI em PT-BR e EN (sem strings fixas no código).
