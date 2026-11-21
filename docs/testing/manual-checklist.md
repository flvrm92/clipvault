# Checklist manual de release

> Roteiro executado **antes de cada release**, com o pacote assinado pelo certificado de teste (specs/08 §2, specs/09 §8). Copie esta lista para o PR da release, marque cada item e anote o ambiente. Item não executado fica como **"não verificado"**, com o motivo; nunca marcado como feito.

## Ambiente

| Campo | Valor |
|-------|-------|
| Versão / commit | |
| Windows (edição e build) | |
| Arquitetura | x64 / arm64 |
| Monitores e escalas | |
| Data e responsável | |

## 1. Pacote e instalação

- [ ] `tools/new-test-cert.ps1` executado e o `.cer` importado em `LocalMachine\TrustedPeople` (uma vez por máquina).
- [ ] `tools/release.ps1 -Version X.Y.Z -Sign` gera `ClipVault_X.Y.Z.0_x64.msix`, `…_arm64.msix`, o `.msixbundle` e o `.msixupload` sem erros.
- [ ] `Add-AppxPackage target\msix\ClipVault_X.Y.Z.0_x64_arm64.msixbundle` instala sem avisos.
- [ ] Menu Iniciar mostra "Clip-Vault" com o ícone da marca; nome e descrição em PT-BR com Windows em português e em EN com Windows em inglês.
- [ ] Primeira execução: o app vai para a bandeja e mostra as boas-vindas (TRAY-006).
- [ ] O log em `logs\` registra "start with Windows: MSIX StartupTask" (identidade de pacote detectada).
- [ ] **WACK**: `appcert.exe test -appxpackagepath <bundle> -reportoutputpath target\msix\wack.xml` → resultado *PASSED*; relatório anexado ao PR.

## 2. Início com o Windows (CFG-003, StartupTask)

- [ ] Aceitar nas boas-vindas → *Configurações do Windows → Apps → Inicialização* mostra "Clip-Vault" ligado.
- [ ] Reiniciar a sessão → o app inicia na bandeja, sem janela.
- [ ] Desligar pela tela do Windows → a seção Geral do ClipVault mostra "Desativado nas configurações de inicialização do Windows. Reative por lá." e não reativa sozinha (`DisabledByUser`).
- [ ] Desligar e ligar pelo ClipVault → o estado do Windows acompanha.

## 3. Dados e desinstalação (PRIV-005)

> Rodar numa máquina ou conta **sem** `%LOCALAPPDATA%\dev.clipvault.desktop`. Se a pasta já existir (build sem pacote), o Windows deixa o pacote usar a pasta real e os itens abaixo não valem (specs/09 §4).

- [ ] *Configurações → Sobre → Pasta de dados → Abrir pasta* abre `%LOCALAPPDATA%\Packages\<PFN>\LocalCache\Local\dev.clipvault.desktop` (com `history.db`, `images\` e `logs\`).
- [ ] Nada é criado em `%LOCALAPPDATA%\dev.clipvault.desktop` fora da pasta do pacote.
- [ ] Atualização: instalar a versão anterior, copiar textos e imagens, criar snippets, instalar a nova por cima → histórico, fixados, snippets e configurações preservados.
- [ ] `Remove-AppxPackage` (ou desinstalar pelo Menu Iniciar) → a pasta `Packages\<PFN>` é removida; a StartupTask some da lista de inicialização.

## 4. Fluxos principais

- [ ] Copiar texto no Bloco de Notas e uma imagem no Paint → ambos no topo do popup (`Ctrl + Win + V`).
- [ ] Buscar, colar com Enter no Bloco de Notas, no Word e no navegador; o foco volta ao app de origem.
- [ ] Snippet com atalho cola o texto e restaura o clipboard anterior.
- [ ] Senha copiada de um gerenciador de senhas (KeePassXC, Bitwarden ou 1Password) não aparece no histórico (CAP-007).
- [ ] App adicionado em *Aplicativos excluídos* não gera itens (CAP-009).
- [ ] Pausar pela bandeja → nada é capturado; retomar → volta a capturar.
- [ ] Trocar idioma e tema nas configurações → popup, bandeja e notificações mudam sem reiniciar.
- [ ] Links do "Sobre" abrem no navegador: política de privacidade, licenças de terceiros e suporte (sem 404).

## 5. Requisitos `[manual]`

- [ ] **PST-006** — colar num app elevado (Bloco de Notas "Executar como administrador"): o conteúdo fica no clipboard e aparece "Conteúdo copiado. Cole com Ctrl+V."
- [ ] **PERF-001** — atalho → popup visível e focado sem atraso perceptível (meta p95 ≤ 150 ms); 20 aberturas seguidas sem falha.
- [ ] **PERF-008** — com 5.000 itens, rolagem contínua sem travamentos.
- [ ] **A11Y-005** — escala do Windows 100%, 150%, 200% e 300% e "Tamanho do texto" 150%: popup, configurações e boas-vindas sem cortes nem sobreposição.
- [ ] **CMP-004** — dois monitores com escalas diferentes (ex.: 100% e 150%): o popup abre junto ao caret (ou ao ponteiro) no monitor certo, inteiro dentro da área de trabalho, nítido e no tamanho certo; arrastar a janela de configurações entre monitores ajusta a escala.
- [ ] **CMP-002** — repetir as seções 1, 2 e 4 num dispositivo arm64 (ou VM arm64).
- [ ] **CMP-001** — repetir as seções 1, 2 e 4 no Windows 10 22H2 (build 19045); popup com fundo sólido (sem Acrylic).
- [ ] **CMP-003** — numa VM sem WebView2 (ou com a chave `pv` renomeada temporariamente): abrir o app mostra a caixa "Não foi possível iniciar o ClipVault" com o endereço de download, e o app encerra.

## 6. Listagem da Store

- [ ] Capturas 1920 × 1080 atualizadas em `packaging/store/screenshots/`: popup claro, popup escuro, busca, snippets, configurações (07 §6.4).
- [ ] Textos de [docs/store/listing.md](../store/listing.md) revisados nos dois idiomas.
- [ ] `MaxVersionTested` do manifesto igual à build do Windows usada nesta checklist (specs/09 §4).
