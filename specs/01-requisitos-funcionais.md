# 01 — Requisitos funcionais

> Status: Aprovada · v1.0 · 2026-11-20
>
> Prioridade MoSCoW: **M** Must · **S** Should · **C** Could · **W** Won't (v1.0).
> Cada critério de aceite (CA) deve ser coberto por ao menos um teste automatizado, salvo indicação `[manual]`.

## Áreas

| Prefixo | Área |
|---------|------|
| `TRAY` | Bandeja do sistema e ciclo de vida |
| `CAP` | Captura do clipboard |
| `HIST` | Histórico (visualização e gestão) |
| `PST` | Colagem |
| `HK` | Atalho global de abertura |
| `SNP` | Snippets (conteúdos fixos com atalho) |
| `RET` | Retenção |
| `CFG` | Configurações gerais |
| `THM` | Tema e aparência |
| `I18N` | Idiomas |

---

## TRAY — Bandeja do sistema e ciclo de vida

### TRAY-001 · Ícone na bandeja — M
O app roda em segundo plano, sem janela na barra de tarefas, com ícone na área de notificação.

```gherkin
Cenário: App iniciado
  Dado que o ClipVault foi iniciado
  Então um ícone do ClipVault aparece na área de notificação
  E nenhum botão do app aparece na barra de tarefas
```

### TRAY-002 · Interações com o ícone — M
- Clique esquerdo: abre/fecha o popup de histórico.
- Clique direito: menu de contexto com: **Abrir histórico**, **Pausar captura / Retomar captura**, **Configurações**, **Sobre**, **Sair**.

```gherkin
Cenário: Menu de contexto
  Quando o usuário clica com o botão direito no ícone da bandeja
  Então o menu exibe "Abrir histórico", "Pausar captura", "Configurações", "Sobre" e "Sair"

Cenário: Clique esquerdo alterna o popup
  Dado que o popup está fechado
  Quando o usuário clica com o botão esquerdo no ícone
  Então o popup de histórico é exibido
```

### TRAY-003 · Indicação de captura pausada — M
Com a captura pausada, o ícone muda para a variante "pausado" e o tooltip indica o estado.

### TRAY-004 · Instância única — M
Uma segunda execução não cria outra instância; ela sinaliza a instância existente para abrir o popup e encerra.

### TRAY-005 · Sair — M
"Sair" desregistra todos os atalhos globais, remove o listener de clipboard, conclui gravações pendentes e encerra o processo.

### TRAY-006 · Primeira execução — S
Na primeira execução, exibe uma janela de boas-vindas curta (1 tela) com o atalho de abertura atual, link para configurações e a opção "Iniciar com o Windows".

---

## CAP — Captura do clipboard

### CAP-001 · Captura de texto — M
Toda alteração do clipboard contendo texto Unicode (`CF_UNICODETEXT`) gera um item de texto no histórico.

```gherkin
Cenário: Texto copiado é registrado
  Dado que a captura está ativa
  Quando qualquer aplicativo coloca o texto "olá mundo" no clipboard
  Então um item de texto "olá mundo" aparece no topo do histórico
  E o item registra data/hora de captura
```

### CAP-002 · Captura de imagem — M
Alterações contendo imagem (`PNG` registrado, `CF_DIBV5` ou `CF_DIB`, nesta ordem de preferência) geram um item de imagem. A imagem é salva como PNG em disco; o SQLite guarda apenas metadados (ver [05](05-modelo-de-dados.md)).

```gherkin
Cenário: Print screen é registrado
  Dado que a captura está ativa
  Quando o usuário copia uma imagem de 1920x1080
  Então um item de imagem aparece no topo do histórico com miniatura
  E o arquivo PNG original é salvo na pasta de imagens do app
  E o banco registra largura 1920, altura 1080, tamanho em bytes e hash
```

### CAP-003 · Prioridade entre formatos — M
Quando o clipboard contém texto **e** imagem (ex.: célula do Excel), registra-se **um único item**, priorizando o texto. Se o texto estiver vazio/só espaços, registra-se a imagem.

### CAP-004 · Deduplicação — M
Se o conteúdo capturado (por hash SHA-256 do conteúdo normalizado) já existir no histórico, **não** se cria novo item: o item existente vai para o topo (`last_used_at` atualizado) e `use_count` é incrementado.

```gherkin
Cenário: Copiar o mesmo texto duas vezes
  Dado que o histórico contém o item "abc" na 5ª posição
  Quando o usuário copia "abc" novamente
  Então o histórico não ganha um novo item
  E "abc" passa a ser o 1º item
```

### CAP-005 · Ignorar conteúdo vazio — M
Texto vazio ou composto apenas por espaços em branco não é registrado.

### CAP-006 · Limites de tamanho — M
- Texto acima do limite configurado (padrão **1 MB**, UTF-8) não é registrado; o evento é logado.
- Imagem acima do limite configurado (padrão **25 MB** em bitmap descomprimido) não é registrada.

### CAP-007 · Respeitar sinalizadores de privacidade — M
Não registrar o conteúdo quando o clipboard contiver qualquer um dos formatos:
`ExcludeClipboardContentFromMonitorProcessing`, `Clipboard Viewer Ignore`, ou `CanIncludeInClipboardHistory` com valor `0` (usados por gerenciadores de senha e pelo próprio Windows).

```gherkin
Cenário: Senha copiada de gerenciador de senhas
  Dado que a captura está ativa
  Quando um gerenciador de senhas copia uma senha marcando "ExcludeClipboardContentFromMonitorProcessing"
  Então nenhum item é adicionado ao histórico
```

> **Nota:** isso é uma exceção deliberada ao requisito "todo conteúdo copiado será registrado". É padrão de mercado e necessário para a aprovação/confiança do usuário.

### CAP-008 · Pausar captura — M
Com a captura pausada (via bandeja ou configurações), alterações do clipboard são ignoradas. O estado persiste entre reinícios.

### CAP-009 · Aplicativos excluídos — S
O usuário pode cadastrar executáveis (ex.: `keepass.exe`) cujas cópias são ignoradas. A identificação usa o processo dono do clipboard (`GetClipboardOwner` → PID → nome do executável); se não for possível identificar o dono, o conteúdo é capturado normalmente.

### CAP-010 · Captura própria — M
Quando o próprio ClipVault escreve no clipboard (para colar um item do histórico ou um snippet), essa alteração **não** cria um item novo:
- colar item do histórico → o item vai para o topo (como CAP-004);
- colar snippet → nada é registrado no histórico.

### CAP-011 · Origem do item — C
Registrar o nome do executável de origem (quando identificável) e exibi-lo no item.

### CAP-012 · Debounce — M
Várias notificações de clipboard em sequência rápida (< 100 ms, comum em apps que escrevem vários formatos) resultam em uma única leitura do conteúdo final.

---

## HIST — Histórico

### HIST-001 · Listagem — M
O popup lista os itens ordenados por: **fixados primeiro**, depois `last_used_at` decrescente. Cada linha exibe:
- texto: prévia de até 3 linhas (whitespace colapsado, cortado com reticências);
- imagem: miniatura proporcional + dimensões (ex.: `1920 × 1080`);
- tempo relativo ("agora", "há 5 min", "ontem"), ícone de fixado quando aplicável.

### HIST-002 · Paginação / rolagem virtual — M
A lista carrega sob demanda (páginas de 50) e usa rolagem virtualizada; deve permanecer fluida com 5.000 itens.

### HIST-003 · Busca — M
Campo de busca com foco automático ao abrir o popup. Filtra itens de texto por substring, sem diferenciar maiúsculas/minúsculas nem acentos (`cafe` encontra `Café`). Resultado atualiza a cada tecla (debounce 80 ms).

```gherkin
Cenário: Busca sem acento
  Dado que o histórico contém "Café com leite" e "Chá"
  Quando o usuário digita "cafe"
  Então apenas "Café com leite" é exibido
```

### HIST-004 · Filtro por tipo — S
Filtros rápidos: **Tudo · Texto · Imagens · Fixados**.

### HIST-005 · Fixar / desafixar — M
O usuário pode fixar um item (botão ou `Ctrl + P`). Itens fixados ficam no topo e não expiram (RET-003).

### HIST-006 · Excluir item — M
O usuário pode excluir um item (botão ou `Delete`). Para imagens, o arquivo em disco também é removido.

### HIST-007 · Limpar histórico — M
Ação "Limpar histórico" (em Configurações → Histórico e no menu do popup) remove todos os itens **não fixados**, com confirmação. Opção adicional "incluir fixados".

### HIST-008 · Pré-visualização — S
Ao focar um item por 600 ms ou pressionar `Espaço`, exibe pré-visualização ampliada (texto completo com rolagem; imagem em tamanho ajustado).

### HIST-009 · Atualização em tempo real — M
Com o popup aberto, novos itens capturados aparecem no topo sem reabrir a janela.

### HIST-010 · Estado vazio — M
Sem itens, o popup exibe ilustração + mensagem ("Nada por aqui ainda. Copie algo para começar.") e o atalho atual. Sem resultados de busca, exibe mensagem específica.

### HIST-011 · Copiar sem colar — S
`Ctrl + C` sobre um item coloca-o no clipboard e fecha o popup, sem colar.

---

## PST — Colagem

### PST-001 · Colar item selecionado — M
Ao selecionar um item (clique, `Enter` ou duplo clique), o ClipVault:
1. coloca o conteúdo no clipboard;
2. fecha o popup;
3. devolve o foco à **janela alvo**;
4. envia `Ctrl + V` à janela alvo.

```gherkin
Cenário: Colar texto do histórico no Bloco de Notas
  Dado que o Bloco de Notas está em foco
  E o usuário abre o popup pelo atalho global
  Quando o usuário seleciona o item "relatório final" e pressiona Enter
  Então o popup fecha
  E o Bloco de Notas volta a ter foco
  E o texto "relatório final" é inserido no Bloco de Notas
  E "relatório final" passa a ser o 1º item do histórico
```

### PST-002 · Colar como texto puro — S
`Shift + Enter` cola o texto do item removendo quebras de linha finais e espaços nas extremidades (útil para formulários). (Na v1 todo texto já é puro; este modo aplica a normalização *trim*.)

### PST-003 · Colar imagem — M
Itens de imagem são colocados no clipboard nos formatos `PNG` e `CF_DIB` antes do `Ctrl + V`, maximizando compatibilidade.

### PST-004 · Colagem rápida por número — S
Com o popup aberto, `Alt + 1` … `Alt + 9` colam o 1º…9º item visível.

### PST-005 · Janela alvo indisponível — M
Se a janela alvo foi fechada ou não pode receber foco, o conteúdo fica no clipboard, o popup fecha e uma notificação discreta informa "Conteúdo copiado. Cole com Ctrl+V."

### PST-006 · Apps elevados — M
Se a janela alvo roda como administrador (UIPI bloqueia `SendInput`), o comportamento é o de PST-005. Documentar a limitação no "Sobre"/FAQ. `[manual]`

### PST-007 · Fechar sem colar — M
`Esc`, clique fora ou perda de foco fecham o popup sem alterar o clipboard.

---

## HK — Atalho global de abertura

### HK-001 · Atalho configurável — M
O usuário define o atalho global que abre o popup. Padrão: **`Ctrl + Win + V`**.

### HK-002 · Captura do atalho na UI — M
Em Configurações → Atalhos, o campo de atalho entra em modo de gravação ao receber foco; a próxima combinação pressionada é exibida (ex.: `Ctrl + Win + V`). `Esc` cancela; `Backspace` limpa.

### HK-003 · Validação — M
Um atalho é válido se:
- tiver **pelo menos um modificador** (`Ctrl`, `Alt`, `Shift`, `Win`) **e** uma tecla não modificadora — exceção: `F13`–`F24` podem ser usadas sem modificador;
- não estiver na lista de combinações reservadas (ex.: `Ctrl+C`, `Ctrl+V`, `Ctrl+X`, `Ctrl+Z`, `Win+L`, `Win+V`, `Ctrl+Alt+Del`, `Alt+F4`, `Alt+Tab`);
- não conflitar com outro atalho do próprio ClipVault (abertura ou snippets).

```gherkin
Cenário: Conflito interno
  Dado que o snippet "Assinatura" usa "Ctrl + Shift + 1"
  Quando o usuário tenta definir "Ctrl + Shift + 1" como atalho de abertura
  Então o atalho não é salvo
  E a mensagem "Atalho já usado pelo snippet 'Assinatura'" é exibida
```

### HK-004 · Conflito com outro aplicativo — M
Se o registro no sistema falhar (atalho em uso por outro app), o atalho anterior é mantido e a UI exibe "Este atalho está em uso por outro aplicativo".

### HK-005 · Posição do popup — M
O popup abre próximo ao cursor de texto (caret) da janela alvo quando obtível; senão, próximo ao ponteiro do mouse. Sempre totalmente dentro da área de trabalho do monitor correspondente.

---

## SNP — Snippets (conteúdos fixos com atalho)

### SNP-001 · Cadastro de snippet — M
Em Configurações → Atalhos → **Conteúdos fixos**, o usuário cria snippets com:
- **Nome** (obrigatório, 1–60 caracteres, único);
- **Conteúdo** (texto, obrigatório, até 100 KB);
- **Atalho** (opcional — sem atalho, o snippet fica acessível só pelo popup, ver SNP-007);
- **Ativo** (liga/desliga sem excluir).

### SNP-002 · Múltiplos snippets — M
Não há limite prático (mínimo suportado: 100) de snippets, cada um com atalho próprio (ex.: `Ctrl + Shift + 1`, `Ctrl + Shift + 2`, …).

### SNP-003 · Colar por atalho — M
Ao pressionar o atalho de um snippet ativo em qualquer aplicativo, o conteúdo é colado na janela em foco.

```gherkin
Cenário: Colar snippet por atalho
  Dado o snippet "Teste" com conteúdo "Teste de texto" e atalho "Ctrl + Shift + V"
  E o Bloco de Notas está em foco
  Quando o usuário pressiona "Ctrl + Shift + V"
  Então "Teste de texto" é inserido no Bloco de Notas
  E nenhum item novo é adicionado ao histórico
```

### SNP-004 · Modificadores pressionados — M
Como o atalho é disparado com modificadores ainda pressionados (ex.: `Shift`), a colagem aguarda a liberação física dos modificadores (timeout 1 s) antes de enviar `Ctrl + V`; se o timeout expirar, envia key-up sintético dos modificadores antes do `Ctrl + V`. Objetivo: evitar que o app alvo receba `Ctrl + Shift + V`.

### SNP-005 · Restaurar clipboard — S
Após colar um snippet, o conteúdo anterior do clipboard é restaurado (após 500 ms, configurável). Configuração "Restaurar área de transferência após colar snippet" — padrão **ligado**.

### SNP-006 · Validação de atalho — M
Mesmas regras de HK-003/HK-004. Edição/exclusão de snippet desregistra/registra atalhos imediatamente.

### SNP-007 · Snippets no popup — S
O popup tem uma aba/filtro **Snippets** que lista os snippets ativos (com seus atalhos); selecionar um cola o conteúdo (mesmo fluxo de PST-001, sem registrar no histórico).

### SNP-008 · Reordenar — C
Arrastar para reordenar snippets na lista de configurações.

### SNP-009 · Criar snippet a partir do histórico — C
Ação "Salvar como snippet" em um item de texto do histórico.

### SNP-010 · Importar / exportar — C
Exportar/importar snippets em JSON versionado.

---

## RET — Retenção

### RET-001 · Período de retenção — M
Opções: **1 dia · 7 dias · 30 dias (padrão) · 90 dias · 1 ano · Para sempre**. Itens com `last_used_at` mais antigo que o período são removidos.

### RET-002 · Quantidade máxima — M
Opções: **100 · 500 · 1.000 (padrão) · 5.000 · Ilimitado**. Ao exceder, removem-se os itens não fixados menos recentes.

### RET-003 · Fixados não expiram — M
Itens fixados não são removidos por RET-001/RET-002 nem contam para o limite de quantidade.

### RET-004 · Execução da limpeza — M
A limpeza ocorre: na inicialização, a cada captura (somente regra de quantidade) e a cada 1 hora (ambas as regras). Alterar a política aplica a limpeza imediatamente, após confirmação se ela remover itens ("Isso removerá 312 itens. Continuar?").

### RET-005 · Arquivos órfãos — M
Na inicialização, arquivos de imagem sem registro no banco são removidos e registros sem arquivo são excluídos (reconciliação).

### RET-006 · Limpar ao sair — C
Opção "Limpar histórico ao sair" (exceto fixados) — padrão desligado.

---

## CFG — Configurações gerais

### CFG-001 · Janela de configurações — M
Janela própria, com seções: **Geral · Histórico · Atalhos · Privacidade · Aparência · Sobre**.

### CFG-002 · Persistência imediata — M
Alterações são salvas e aplicadas imediatamente (sem botão "Salvar"), exceto edição de snippet, que tem Salvar/Cancelar.

### CFG-003 · Iniciar com o Windows — M
Opção "Iniciar com o Windows" — implementada via `StartupTask` do MSIX (ver [09](09-distribuicao-msix.md)). Padrão: ligado após confirmação na primeira execução. Se o usuário desabilitou nas Configurações do Windows, a opção fica desabilitada com explicação.

### CFG-004 · Restaurar padrões — S
Botão "Restaurar padrões" por seção (não afeta histórico nem snippets).

### CFG-005 · Sobre — M
Versão, link para política de privacidade, licenças de terceiros, link de suporte, e "Abrir pasta de dados" (somente leitura, para diagnóstico).

### CFG-006 · Configurações válidas — M
Valores inválidos/corrompidos lidos do armazenamento são substituídos pelo padrão e logados; o app nunca deixa de iniciar por configuração inválida.

---

## THM — Tema e aparência

### THM-001 · Modo de tema — M
Opções: **Sistema (padrão) · Claro · Escuro**. Em "Sistema", segue `AppsUseLightTheme` e reage a mudanças em tempo real, sem reiniciar.

### THM-002 · Material de fundo — S
No Windows 11, popup e configurações usam material **Mica** (configurações) / **Acrylic** (popup); no Windows 10 ou com transparência desativada, cor sólida equivalente.

### THM-003 · Ícone da bandeja adaptado — M
O ícone da bandeja usa a variante clara ou escura conforme o tema da barra de tarefas (`SystemUsesLightTheme`), e atualiza ao mudar.

### THM-004 · Cor de destaque — C
Opção de usar a cor de destaque do Windows no lugar da cor da marca.

---

## I18N — Idiomas

### I18N-001 · Idiomas suportados — M
PT-BR e EN (fallback EN). Idioma padrão: idioma de exibição do Windows se suportado; senão EN.

### I18N-002 · Troca de idioma — S
Configurações → Geral → Idioma: **Sistema · Português (Brasil) · English**. Aplica sem reiniciar.

### I18N-003 · Formatos locais — M
Datas, tempos relativos e números seguem o idioma ativo.

### I18N-004 · Sem textos fixos no código — M
Nenhuma string visível ao usuário fica fixa no código; todas vêm dos arquivos de tradução. Um teste verifica que as chaves de PT-BR e EN são idênticas.
