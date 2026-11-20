# ADR-0002 · Arquitetura hexagonal em workspace multi-crate

- **Status:** Aceita (2026-11-20)

## Contexto
Requisitos de TDD, SOLID e testes abrangentes, com forte dependência de APIs do Windows (clipboard, hotkeys, SendInput), que não são testáveis em unidade.

## Decisão
Workspace Cargo com crates `domain`, `application`, `infrastructure`, `test-support` e o binário `app` (Tauri). Portas (traits) em `application`, adaptadores em `infrastructure`, ligação no composition root (`app`). A separação em crates **impõe** a regra de dependência via compilador.

## Alternativas consideradas
- **Crate única com módulos** — mais simples, mas a regra de dependência vira só convenção.
- **Mais granularidade (uma crate por adaptador)** — compilação incremental melhor, mas overhead prematuro; pode evoluir depois.

## Consequências
- \+ `domain`/`application` testáveis em qualquer SO, rápidos e determinísticos.
- \+ Troca de adaptadores (ex.: FTS5, criptografia) localizada.
- − Mais boilerplate (portas, DTOs, mapeamentos).
