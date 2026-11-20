# ClipVault

Gerenciador de área de transferência para Windows: histórico de textos e imagens, busca instantânea, atalho global e conteúdos fixos colados por atalho. Escrito em Rust (Tauri 2), distribuído pela Microsoft Store.

> **Status:** M0 — Fundações ([roadmap](specs/10-roadmap.md)). Comece por [specs/README.md](specs/README.md).

## Documentação

- [Especificações (SDD)](specs/README.md)
- [Decisões de arquitetura (ADRs)](docs/adr/README.md)

## Estrutura

```
crates/domain           entidades e regras (sem IO)
crates/application      casos de uso e portas (traits)
crates/infrastructure   adaptadores: SQLite, arquivos, Win32
crates/test-support     fakes e builders para testes
app/                    composition root Tauri (bandeja, janelas, comandos)
ui/                     frontend Svelte 5 + TypeScript
xtask/                  verificações do projeto (arquitetura, rastreabilidade)
specs/ · docs/adr/      especificações e decisões
```

## Pré-requisitos

- Windows 10 22H2+ ou Windows 11
- Rust (a versão é fixada em `rust-toolchain.toml` e instalada automaticamente pelo `rustup`)
- Visual Studio Build Tools com **Desenvolvimento para desktop com C++** (MSVC x64 e ARM64 + Windows SDK)
- Node.js ≥ 24 e pnpm 12 (`corepack enable`)

## Comandos

| Comando | O que faz |
|---------|-----------|
| `pnpm install` | Instala dependências do frontend e o Tauri CLI |
| `pnpm dev` | Roda o app em modo desenvolvimento (Vite + Tauri) |
| `pnpm build:app` | Compila o executável sem empacotar (`target/release/ClipVault.exe`) |
| `cargo test --workspace` | Testes Rust |
| `cargo clippy --workspace --all-targets` | Lint Rust |
| `cargo xtask check-arch` | Verifica a regra de dependência entre camadas (MNT-001) |
| `cargo xtask trace-reqs` | Rastreabilidade requisito ↔ teste |
| `cargo llvm-cov --workspace --exclude clipvault-app` | Cobertura Rust |
| `pnpm --dir ui test` / `lint` / `check` / `coverage` | Testes, lint, typecheck e cobertura do frontend |
| `cargo deny check` | Auditoria de licenças e vulnerabilidades |

## Solução de problemas

**`LNK1104: cannot open file 'msvcrt.lib'`**: o rustc usa a instalação **mais recente** do Visual Studio, inclusive versões Preview/Insiders. Se essa instalação não tiver o componente de C++ para desktop, o link falha mesmo com outra versão completa instalada. Corrija instalando o componente *MSVC x64/x86* na versão mais nova, ou rode o cargo a partir do *Developer PowerShell* da versão completa.
