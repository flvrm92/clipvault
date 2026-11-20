# ADR-0006 · Frontend em Svelte 5 + TypeScript

- **Status:** Aceita (2026-11-20)

## Contexto
Com Tauri, a UI roda em WebView2. O frontend é só apresentação, mas precisa de boa testabilidade, desempenho na lista virtualizada e bundle pequeno.

## Decisão
**Svelte 5 + TypeScript + Vite**, testes com **Vitest + Testing Library**, tipos de DTO gerados a partir do Rust (`specta`/`ts-rs`).

## Alternativas consideradas
- **Leptos (Rust → WASM)** — mantém a UI também em Rust; ecossistema de testes de componente e bibliotecas de UI menos maduro, build mais lento.
- **React** — ecossistema enorme, porém runtime e bundle maiores para uma UI pequena.
- **SolidJS** — excelente desempenho; ecossistema menor que Svelte/React.

## Consequências
- \+ Pouco código, reatividade simples, bundle pequeno, ferramentas de teste maduras.
- − Segunda linguagem no projeto (TS) — mitigada mantendo toda regra de negócio no Rust.
