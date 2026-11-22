# ADR-0010 · Notificações nativas por toast WinRT próprio

- **Status:** Aceita (2026-11-22)

## Contexto
PST-005, PST-006 e HK-004 mostram uma notificação nativa ("Conteúdo copiado. Cole com Ctrl+V.", atalhos ocupados). Até a 1.0.0 isso passava pelo `tauri-plugin-notification` 2.8.1. Na checklist do M7, com o pacote instalado, nenhuma notificação apareceu, embora o log confirmasse a chamada e a detecção de UIPI (`reason=TargetElevated`).

Causa: o Windows só exibe um toast cujo AppUserModelID (AUMID) pertence a um app registrado. Fora de `target/debug|release`, o plugin usa o `identifier` do Tauri (`dev.clipvault.desktop`) como AUMID. Esse ID não existe para o Windows, então o toast é descartado sem erro. O AUMID de um app empacotado é `<PFN>!<AppId>` (aqui `flvrm92.Clip-Vault_2nrc79jzqdzh4!ClipVault`), e o plugin não permite informá-lo.

## Decisão
Um adaptador próprio na infraestrutura (`win32::toast`) mostra o toast pela API WinRT `Windows.UI.Notifications` do crate `windows`, que já é dependência. O AUMID é escolhido na hora:

- **Com identidade de pacote:** o valor de `GetCurrentApplicationUserModelId`.
- **Sem pacote** (desenvolvimento): o AUMID registrado da PowerShell, o mesmo fallback que o plugin usava em `target/`.

O XML do toast é montado com escape de `& < > " '`, porque o texto vem das traduções. O `tauri-plugin-notification` sai do projeto.

## Alternativas consideradas
- **Manter o plugin e aguardar uma correção upstream:** a 1.0.0 sairia sem as notificações exigidas por PST-005, PST-006 e HK-004.
- **`tauri-winrt-notification` direto:** aceita o AUMID, mas traz outra versão do crate `windows` e não cobre a escolha do AUMID. A chamada WinRT em si tem poucas linhas.
- **Registrar o `identifier` como AUMID (atalho no Menu Iniciar com `System.AppUserModel.ID`):** é o mecanismo de apps sem pacote. No MSIX o atalho é do pacote e o AUMID é fixo.

## Consequências
- \+ As notificações aparecem no app da Store com o nome e o ícone do pacote, e no desenvolvimento como "Windows PowerShell".
- \+ Uma dependência a menos (o plugin trazia `notify-rust`, `zbus` e companhia para outras plataformas).
- − O caminho empacotado só é verificável com o pacote instalado (checklist manual: PST-005/PST-006/HK-004).
