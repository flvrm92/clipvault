# ADR-0007 · Empacotamento MSIX por script com Windows SDK

- **Status:** Aceita (2026-11-20)

## Contexto
A distribuição é pela Microsoft Store com MSIX. O Tauri 2 gera MSI/NSIS, não MSIX.

## Decisão
Compilar com `cargo tauri build --no-bundle` e empacotar com script PowerShell (`packaging/msix/build.ps1`) usando `MakePri`, `MakeAppx` e `SignTool` do Windows SDK, a partir de um manifesto template versionado. Bundle x64 + arm64; envio como `.msixupload`.

## Alternativas consideradas
- **Submeter MSI/EXE à Store (Win32 não empacotado)** — possível, mas perde StartupTask, limpeza na desinstalação, atualizações gerenciadas e identidade de pacote.
- **MSIX Packaging Tool (GUI)** — não reprodutível em CI.
- **Ferramentas de terceiros** — dependência extra sem ganho claro.

## Consequências
- \+ Build reprodutível em CI, controle total do manifesto.
- − Script próprio para manter; validado com WACK a cada release.
