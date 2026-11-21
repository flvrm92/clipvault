# 09 — Distribuição (Microsoft Store / MSIX)

> Status: Aprovada · v1.0 · 2026-11-20

## 1. Estratégia

- Distribuição pela **Microsoft Store** como pacote **MSIX** (app Win32 *full trust* empacotado).
- O Tauri 2 não gera MSIX nativamente: o binário é compilado com `tauri build --no-bundle` e empacotado por script próprio com as ferramentas do Windows SDK (`MakePri`, `MakeAppx`, `SignTool`). Ver ADR-0007.
- A Store assina o pacote na publicação; assinatura local (certificado de teste) apenas para sideload/QA.
- Arquiteturas: `x64` e `arm64` em um `.msixbundle`; envio como `.msixupload`.

## 2. Pré-requisitos no Partner Center

| Item | Ação |
|------|------|
| Reserva de nome | Reservar **"ClipVault"** (ter alternativas: "ClipVault — Clipboard Manager", "Clip Vault"). |
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
                         Square71x71Logo="Assets\SmallTile.png"
                         Square310x310Logo="Assets\LargeTile.png" />
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

- Sem identidade de pacote (desenvolvimento), o mesmo fluxo usa a chave `Run` do usuário ([ADR-0008](../docs/adr/0008-inicio-com-windows.md)); a versão empacotada remove esse valor na inicialização, se existir.
- `Enabled="false"`: o início automático só é ativado após o consentimento do usuário (primeira execução — TRAY-006/CFG-003) via `StartupTask.RequestEnableAsync()`. Se o usuário desabilitar em *Configurações do Windows → Apps → Inicialização*, o estado vira `DisabledByUser` e o app não pode reativar sozinho — a UI explica (CFG-003).
- O app inicia sempre direto na bandeja, então não há distinção de comportamento ao ser iniciado pela StartupTask.
- `MaxVersionTested` atualizado a cada release conforme a versão do Windows em que os testes manuais foram feitos.
- Dados do app: com MSIX, gravações em `%LOCALAPPDATA%\<identifier>` são virtualizadas para `%LOCALAPPDATA%\Packages\<PFN>\LocalCache\Local\…` e removidas na desinstalação (PRIV-005). Teste manual na checklist.

## 5. Pipeline de build

```
tools/release.ps1 -Version 1.0.0
  1. Sincroniza versão: Cargo.toml / tauri.conf.json / package.json = 1.0.0; MSIX = 1.0.0.0
     (Store exige revisão = 0)
  2. pnpm --dir ui build
  3. cargo tauri build --no-bundle --target x86_64-pc-windows-msvc
     cargo tauri build --no-bundle --target aarch64-pc-windows-msvc
  4. tools/gen-assets --check
  5. Para cada arch: monta layout/ (ClipVault.exe + Assets + Strings + manifesto renderizado)
     → MakePri new (resources.pri)
     → MakeAppx pack → ClipVault_1.0.0.0_<arch>.msix
  6. MakeAppx bundle → ClipVault_1.0.0.0.msixbundle
  7. Gera .msixupload (zip do bundle [+ símbolos .appxsym])
  8. (QA) SignTool com certificado de teste → instalar via Add-AppxPackage
  9. Windows App Certification Kit (WACK) — deve passar sem falhas
```

- CI executa os passos 1–7 em tags `v*` e anexa artefatos ao release; submissão ao Partner Center manual na v1 (automação via Microsoft Store Submission API/`msstore` CLI é candidata futura).

## 6. Justificativa da capability `runFullTrust` (texto para a submissão)

> ClipVault is a clipboard manager. It requires full trust to (1) monitor clipboard changes system-wide using AddClipboardFormatListener, (2) register user-configured global keyboard shortcuts with RegisterHotKey, and (3) paste the selected item into the previously focused application using SendInput. All data stays on the device; the app makes no network connections.

## 7. Listagem da Store (rascunho)

| Campo | PT-BR | EN |
|-------|-------|----|
| Nome | ClipVault | ClipVault |
| Descrição curta | Histórico de área de transferência rápido e privado, com atalhos para seus textos favoritos. | Fast, private clipboard history with shortcuts for your favorite snippets. |
| Recursos (bullets) | Histórico de textos e imagens · Busca instantânea · Atalho global configurável · Conteúdos fixos com atalhos · Retenção configurável · Tema claro e escuro · 100% local | Text & image history · Instant search · Configurable global shortcut · Snippets with hotkeys · Configurable retention · Light & dark themes · 100% local |
| Categoria | Produtividade | Productivity |
| Palavras-chave | clipboard, área de transferência, histórico, colar, snippets, atalhos | clipboard, history, paste, snippets, hotkeys, copy |

## 8. Checklist de release

- [ ] Versão sincronizada e changelog atualizado.
- [ ] Todos os testes e cobertura verdes; E2E verde.
- [ ] Checklist manual executada (inclui apps elevados, multi-monitor, DPI 100/150/200%, Win10 e Win11, x64 e arm64).
- [ ] WACK aprovado.
- [ ] Instalação/atualização/desinstalação limpas (dados removidos na desinstalação).
- [ ] Atualização a partir da versão anterior preserva histórico e snippets (migração).
- [ ] Capturas de tela e textos da listagem atualizados.
