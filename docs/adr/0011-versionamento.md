# ADR-0011 · Versão incrementada em cada mudança, verificada contra as tags

- **Status:** Aceita (2026-11-23)

## Contexto
O app está publicado na Microsoft Store como 1.0.0 (tag `v1.0.0`). A Store recusa um pacote cuja versão já foi enviada. Até aqui a versão só mudava quando `tools/release.ps1 -Version` era executado. Nenhum PR precisava incrementá-la, e o primeiro PR depois da 1.0.0 (HK-006) chegou a ser aberto ainda declarando 1.0.0.

## Decisão
- Cada PR que muda o app traz a nova versão em `[workspace.package]` do `Cargo.toml`, acima da maior tag `vX.Y.Z`, com a seção correspondente no `CHANGELOG.md` (specs/02 MNT-006, specs/09 §9).
- `cargo xtask check-version` verifica a regra. As versões publicadas vêm das tags git; o commit que carrega a própria tag passa (é o commit do release). O job Rust do CI busca o histórico completo (`fetch-depth: 0`) para ter as tags.
- O incremento segue SemVer: patch para correção, minor para requisito novo, major para incompatibilidade.

## Alternativas consideradas
- **Incrementar só no release (`release.ps1 -Version`):** era o fluxo da 1.0.0. Funciona, mas a `main` passa a ter mudanças sob uma versão já publicada, e esquecer o incremento só aparece na submissão.
- **Derivar a versão do número de build do CI:** garante unicidade, mas a versão deixa de dizer o que mudou e o `Cargo.toml` deixa de ser a fonte única.
- **Versão publicada registrada em arquivo:** dispensa as tags no CI, mas duplica a informação que a tag de release já guarda.

## Consequências
- \+ A versão da `main` nunca coincide com uma versão já enviada à Store.
- \+ O CHANGELOG acompanha cada mudança, não só o release.
- − PRs paralelos antes de um envio disputam a mesma linha do `Cargo.toml`; o segundo resolve o conflito mantendo a versão já incrementada.
- − Sem as tags (clone raso, fork sem tags), a verificação não tem com o que comparar e passa; o CI evita isso com `fetch-depth: 0`.
