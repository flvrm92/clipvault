# ADR-0009 · Avisos de licenças de terceiros gerados por ferramenta

- **Status:** Aceita (2026-11-21)

## Contexto
CFG-005 tem um link "Licenças de terceiros" que aponta para `THIRD-PARTY-NOTICES.md` na `main`. O pacote MSIX distribui código de centenas de crates e de alguns pacotes npm (Svelte e dependências de runtime da UI). Muitas licenças (MIT, BSD, Apache-2.0) exigem reproduzir o texto da licença e o aviso de copyright. Manter isso à mão é inviável e fica desatualizado a cada `cargo update`.

## Decisão
O arquivo é **gerado** e versionado na raiz:

- **Crates:** `cargo about generate` com `about.toml` (licenças aceitas, alinhadas ao `deny.toml`) e o template `about.hbs`, considerando apenas o grafo de dependências do binário (`clipvault-app`) para o target Windows.
- **UI:** `pnpm licenses list --prod --json` (apenas dependências de produção que vão para o bundle), convertido em Markdown pelo mesmo script. Por isso o `svelte`, cujo runtime é compilado para dentro do bundle, fica em `dependencies` (e não em `devDependencies`) no `ui/package.json`.
- **Script:** `tools/third-party-notices.ps1` junta as duas partes em `THIRD-PARTY-NOTICES.md`. É rodado pelo `tools/release.ps1`; o diff do arquivo aparece no PR da release.

`cargo-about` é ferramenta de desenvolvimento (instalada com `cargo install --locked cargo-about --features cli`), não dependência do app. As crates do próprio projeto (`publish = false`) são ignoradas (`private = { ignore = true }`); a geração usa `--offline` para depender só do `Cargo.lock` e do cache local.

## Alternativas consideradas
- **Arquivo escrito à mão** — desatualiza a cada atualização de dependência; alto risco de omissão.
- **`cargo-bundle-licenses`** — menos usado; sem template, formato de saída menos legível.
- **Gerar só a lista de nomes e licenças, sem os textos** — não atende às licenças que exigem reproduzir o texto.

## Consequências
- \+ Avisos completos e reproduzíveis a partir do `Cargo.lock` e do `pnpm-lock.yaml`.
- \+ As licenças aceitas ficam declaradas em dois lugares coerentes (`deny.toml` para bloquear, `about.toml` para gerar).
- − Uma ferramenta a mais para instalar na máquina de release e no CI de release.
