# 09 — Distribuição (Microsoft Store / MSIX)

> Status: Aprovada · v1.1 · 2026-11-23

## 1. Estratégia

- Distribuição pela **Microsoft Store** como pacote **MSIX** (app Win32 *full trust* empacotado).
- O Tauri 2 não gera MSIX nativamente: o binário é compilado com `pnpm tauri build --no-bundle` (Tauri CLI do workspace pnpm) e empacotado por script próprio com as ferramentas do Windows SDK (`MakePri`, `MakeAppx`, `SignTool`). Ver ADR-0007.
- A Store assina o pacote na publicação; assinatura local (certificado de teste) apenas para sideload/QA.
- Arquiteturas: `x64` e `arm64` em um `.msixbundle`; envio como `.msixupload`.

## 2. Pré-requisitos no Partner Center

| Item | Ação |
|------|------|
| Reserva de nome | Nome reservado: **"Clip-Vault"**. O `AppDisplayName` dos `resources.resw` (Menu Iniciar e Store) e o `StartupTaskDisplayName` usam exatamente esse nome — a Store rejeita pacote cujo nome de exibição não é um nome reservado. Dentro do app a marca continua "ClipVault". |
| Identidade do pacote | Copiar `Package/Identity/Name`, `Publisher` (`CN=…`) e `PublisherDisplayName` de *Gerenciamento de produto → Identidade do produto* para `packaging/msix/identity.json` (não hardcodar no manifesto; o script injeta). |
| Capability restrita | `runFullTrust` exige justificativa na submissão — texto pronto na §6. |
| Política de privacidade | URL pública obrigatória (PRIV-006). |
| Classificação etária | Questionário IARC (app utilitário, sem conteúdo gerado por terceiros nem comunicação online). |
| Mercados / preço | **Grátis**, todos os mercados disponíveis (ver [10](10-roadmap.md#decisões-de-produto)). |

## 3. Estrutura de arquivos

```
packaging/msix/
├─ AppxManifest.template.xml     # template com placeholders {{NAME}}, {{PUBLISHER}}, {{VERSION}}, {{ARCH}}
├─ identity.json                 # valores do Partner Center (versionado; não é segredo)
├─ priconfig.xml                 # config do MakePri (recursos por idioma e escala)
├─ Strings/
│  ├─ pt-BR/resources.resw       # DisplayName, Description, StartupTask DisplayName
│  └─ en-US/resources.resw
├─ Assets/                       # gerados por tools/gen-assets (ver 07 §6.3)
└─ build.ps1                     # orquestra o empacotamento
```

## 4. Manifesto (template)

```xml
<?xml version="1.0" encoding="utf-8"?>
<Package
  xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:desktop="http://schemas.microsoft.com/appx/manifest/desktop/windows10"
  xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
  IgnorableNamespaces="uap desktop rescap">

  <Identity Name="{{NAME}}" Publisher="{{PUBLISHER}}" Version="{{VERSION}}" ProcessorArchitecture="{{ARCH}}" />

  <Properties>
    <DisplayName>ms-resource:AppDisplayName</DisplayName>
    <PublisherDisplayName>{{PUBLISHER_DISPLAY_NAME}}</PublisherDisplayName>
    <Logo>Assets\StoreLogo.png</Logo>
  </Properties>

  <Dependencies>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.19041.0" MaxVersionTested="10.0.26100.0" />
  </Dependencies>

  <Resources>
    <Resource Language="en-US" />
    <Resource Language="pt-BR" />
  </Resources>

  <Applications>
    <Application Id="ClipVault" Executable="ClipVault.exe" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements
        DisplayName="ms-resource:AppDisplayName"
        Description="ms-resource:AppDescription"
        BackgroundColor="transparent"
        Square150x150Logo="Assets\Square150x150Logo.png"
        Square44x44Logo="Assets\Square44x44Logo.png">
        <uap:DefaultTile Wide310x150Logo="Assets\Wide310x150Logo.png"
                         Square71x71Logo="Assets\Square71x71Logo.png"
                         Square310x310Logo="Assets\Square310x310Logo.png" />
        <uap:SplashScreen Image="Assets\SplashScreen.png" />
      </uap:VisualElements>

      <Extensions>
        <!-- CFG-003: iniciar com o Windows -->
        <desktop:Extension Category="windows.startupTask">
          <desktop:StartupTask TaskId="ClipVaultStartup" Enabled="false"
                               DisplayName="ms-resource:StartupTaskDisplayName" />
        </desktop:Extension>
      </Extensions>
    </Application>
  </Applications>

  <Capabilities>
    <rescap:Capability Name="runFullTrust" />
  </Capabilities>
</Package>
```

### Notas

- Sem identidade de pacote (desenvolvimento), o mesmo fluxo usa a chave `Run` do usuário ([ADR-0008](../docs/adr/0008-inicio-com-windows.md)). A versão empacotada não consegue remover esse valor, porque as escritas em `HKCU` são virtualizadas; a instância única (TRAY-004) evita duas cópias rodando.
- `Enabled="false"`: o início automático só é ativado após o consentimento do usuário (primeira execução — TRAY-006/CFG-003) via `StartupTask.RequestEnableAsync()`. Se o usuário desabilitar em *Configurações do Windows → Apps → Inicialização*, o estado vira `DisabledByUser` e o app não pode reativar sozinho — a UI explica (CFG-003).
- O app inicia sempre direto na bandeja, então não há distinção de comportamento ao ser iniciado pela StartupTask.
- Notificações (PST-005, PST-006, HK-004): o Windows só exibe um toast com o AUMID de um app registrado. Empacotado, o app usa o AUMID do pacote (`GetCurrentApplicationUserModelId`, `<PFN>!ClipVault`); sem pacote, o da PowerShell ([ADR-0010](../docs/adr/0010-notificacoes-toast.md)).
- `MaxVersionTested` atualizado a cada release conforme a versão do Windows em que os testes manuais foram feitos.
- Dados do app: com MSIX, gravações em `%LOCALAPPDATA%\<identifier>` são virtualizadas para `%LOCALAPPDATA%\Packages\<PFN>\LocalCache\Local\…` e removidas na desinstalação (PRIV-005). **Exceção:** se a pasta `%LOCALAPPDATA%\dev.clipvault.desktop` já existir (máquina que rodou uma build sem pacote), o Windows deixa o pacote usar a pasta real, e ela fica para trás na desinstalação. Só afeta máquinas de desenvolvimento; a checklist manual roda numa máquina ou conta sem essa pasta.

## 5. Pipeline de build

```
tools/release.ps1 -Version 1.0.0
  1. Sincroniza versão: [workspace.package] do Cargo.toml = 1.0.0 (tauri.conf.json herda do Cargo;
     os package.json não têm versão) + cargo update --workspace; MSIX = 1.0.0.0 (Store exige revisão = 0)
  2. cargo run -p gen-assets -- --check; tools/third-party-notices.ps1 (ADR-0009)
  3. pnpm tauri build --no-bundle --target x86_64-pc-windows-msvc    (o beforeBuildCommand compila a UI)
     pnpm tauri build --no-bundle --target aarch64-pc-windows-msvc
  4. packaging/msix/build.ps1 — para cada arch: monta target/msix/layout-<arch>/
     (ClipVault.exe + Assets + Strings + manifesto renderizado a partir de identity.json)
     → MakePri new (resources.pri)
     → MakeAppx pack → ClipVault_1.0.0.0_<arch>.msix
  5. MakeAppx bundle → ClipVault_1.0.0.0_x64_arm64.msixbundle
  6. Gera .msixupload (zip do bundle + símbolos .appxsym com os PDBs)
  7. (QA) tools/new-test-cert.ps1 cria um certificado autoassinado com subject = Publisher do
     identity.json; build.ps1 -Sign assina com SignTool → instalar via Add-AppxPackage
  8. Windows App Certification Kit (WACK) — deve passar sem falhas
```

- As ferramentas do SDK são localizadas pela versão mais nova em `Windows Kits\10\bin` (sem caminho fixo).
- CI executa os passos 1–6 em tags `v*` e anexa os artefatos a um release em rascunho; nos PRs, um job empacota só x64 (sem release) para pegar quebras do script. Submissão ao Partner Center manual na v1 (automação via Microsoft Store Submission API/`msstore` CLI é candidata futura).
- O teste `cmp_001_…` (xtask) valida o template do manifesto: `MinVersion`, `runFullTrust`, `TaskId` e a existência de todo asset referenciado.

## 6. Justificativa da capability `runFullTrust` (texto para a submissão)

> ClipVault is a clipboard manager. It requires full trust to (1) monitor clipboard changes system-wide using AddClipboardFormatListener, (2) register user-configured global keyboard shortcuts with RegisterHotKey, and (3) paste the selected item into the previously focused application using SendInput. All data stays on the device; the app makes no network connections.

## 7. Listagem da Store (rascunho)

| Campo | PT-BR | EN |
|-------|-------|----|
| Nome | Clip-Vault | Clip-Vault |
| Descrição curta | Histórico de área de transferência rápido e privado, com atalhos para seus textos favoritos. | Fast, private clipboard history with shortcuts for your favorite snippets. |
| Recursos (bullets) | Histórico de textos e imagens · Busca instantânea · Atalho global configurável · Conteúdos fixos com atalhos · Retenção configurável · Tema claro e escuro · 100% local | Text & image history · Instant search · Configurable global shortcut · Snippets with hotkeys · Configurable retention · Light & dark themes · 100% local |
| Categoria | Produtividade | Productivity |
| Palavras-chave | clipboard, área de transferência, histórico, colar, snippets, atalhos | clipboard, history, paste, snippets, hotkeys, copy |

## 8. Checklist de release

- [ ] Versão sincronizada e changelog datado (a seção `## [X.Y.Z] - Unreleased` recebe a data do envio).
- [ ] Todos os testes e cobertura verdes; E2E verde.
- [ ] Checklist manual executada (inclui apps elevados, multi-monitor, DPI 100/150/200%, Win10 e Win11, x64 e arm64).
- [ ] WACK aprovado.
- [ ] Instalação/atualização/desinstalação limpas (dados removidos na desinstalação).
- [ ] Atualização a partir da versão anterior preserva histórico e snippets (migração).
- [ ] Capturas de tela e textos da listagem atualizados.

## 9. Versionamento

- **Fonte única:** `[workspace.package].version` do `Cargo.toml` (`X.Y.Z`). O `tauri.conf.json` herda; o MSIX usa `X.Y.Z.0`, porque a Store exige revisão `0`.
- **Publicadas:** cada envio à Store corresponde a uma tag `vX.Y.Z` no commit empacotado (dispara o workflow de release, §5).
- **Toda mudança sobe a versão (MNT-006).** O PR que muda o app já traz a nova versão, acima da maior tag. Vários PRs antes do próximo envio compartilham a mesma versão ainda não publicada; o primeiro deles faz o incremento. O PR atualiza também a seção `## [X.Y.Z] - Unreleased` do `CHANGELOG.md` (o envio troca `Unreleased` pela data).
- **Qual número sobe** (SemVer):
  - **patch** (`1.0.0 → 1.0.1`): correção sem requisito novo;
  - **minor** (`1.0.x → 1.1.0`): requisito novo ou mudança de comportamento compatível;
  - **major** (`1.x → 2.0.0`): mudança incompatível, como dados que a versão anterior não lê mais.

  Se a versão não publicada já tem o incremento suficiente (ex.: já é `1.1.0` e o PR é uma correção), ela fica como está.
- Para subir: editar a versão no `Cargo.toml`, rodar `cargo update --workspace` (atualiza o `Cargo.lock`) e `cargo xtask check-version`.
