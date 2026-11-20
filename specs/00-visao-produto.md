# 00 — Visão do produto

> Status: Aprovada · v1.0 · 2026-11-20

## 1. Problema

O histórico nativo do Windows (`Win + V`) é limitado: guarda até 25 itens, perde tudo ao reiniciar (exceto fixados), não tem busca, não permite configurar retenção e não oferece colagem de textos recorrentes por atalho. Quem copia e cola o dia inteiro (desenvolvedores, suporte, atendimento, redatores) perde tempo procurando o que copiou ou digitando os mesmos textos repetidamente.

## 2. Proposta

**ClipVault** é um gerenciador de área de transferência para Windows que:

1. Vive na **bandeja do sistema** e registra automaticamente **textos e imagens** copiados.
2. Abre um **popup de histórico** por atalho global configurável (padrão `Ctrl + Win + V`), com busca e navegação por teclado; ao escolher um item, ele é **colado** na janela em que o usuário estava.
3. Mantém o histórico pelo **período e quantidade** que o usuário definir, armazenado localmente (SQLite + arquivos de imagem).
4. Permite cadastrar **conteúdos fixos** (snippets) associados a **atalhos globais** (ex.: `Ctrl + Shift + 1` cola "Atenciosamente, Fulano").
5. Tem visual moderno, simples, com **tema claro/escuro** seguindo o sistema ou a escolha do usuário.
6. É distribuído pela **Microsoft Store** (MSIX).

## 3. Público-alvo

| Persona | Necessidade principal |
|---------|-----------------------|
| **Dev / técnico** | Reaproveitar trechos de código, comandos, IDs, URLs copiados horas antes. |
| **Suporte / atendimento** | Colar respostas padrão por atalho, sem abrir ferramenta. |
| **Usuário de escritório** | Recuperar algo que copiou e sobrescreveu sem querer; colar prints. |

## 4. Escopo da v1.0

- Captura automática de **texto puro** (Unicode) e **imagens** (bitmap/PNG).
- Histórico com busca, filtro por tipo, fixar (pin), excluir, limpar.
- Colar ao selecionar (com retorno de foco à janela anterior).
- Colar como texto puro.
- Atalho global configurável para abrir o histórico.
- Snippets de texto com atalhos globais configuráveis (N snippets).
- Retenção configurável (tempo e quantidade máxima); itens fixados não expiram.
- Pausar/retomar a captura pela bandeja.
- Respeito aos sinalizadores de privacidade do Windows (gerenciadores de senha).
- Iniciar com o Windows (opcional).
- Temas claro/escuro/sistema; interface em PT-BR e EN.
- Pacote MSIX pronto para a Microsoft Store.

## 5. Fora de escopo (v1.0)

| Item | Motivo / quando |
|------|-----------------|
| Rich text (HTML/RTF) preservando formatação | Na v1 o texto é salvo como texto puro. Na **v2**, preservar HTML/RTF será uma opção configurável (o usuário liga ou desliga). |
| Arquivos copiados no Explorer (`CF_HDROP`) | Avaliar na v1.x. |
| Sincronização entre dispositivos / nuvem | Privacidade e escopo; não planejado. |
| Snippets com imagem ou variáveis dinâmicas (data, cursor) | Candidato à v1.x (o modelo já deve permitir extensão). |
| Criptografia do banco local | Avaliar na v1.x (ver [02](02-requisitos-nao-funcionais.md) PRIV). |
| macOS / Linux | Produto é Windows-only. |
| Telemetria | Não haverá coleta de dados de uso. |

## 6. Métricas de sucesso

- Abrir o popup e colar um item em **< 2 s** de interação, só com teclado.
- Zero perda de itens capturados em uso normal (incluindo reinício do PC).
- Avaliação ≥ 4,5 na Store; nenhum crash reportado com taxa > 0,1% das sessões (via relatórios da Partner Center).

## 7. Glossário

| Termo | Definição |
|-------|-----------|
| **Item / Clip** | Um registro do histórico (texto ou imagem). Código: `ClipItem`. |
| **Histórico** | Conjunto de itens capturados e ainda não expirados/excluídos. |
| **Popup** | Janela compacta de histórico aberta pelo atalho global ou pela bandeja. |
| **Fixar (pin)** | Marcar item para nunca expirar e aparecer no topo. |
| **Snippet / Conteúdo fixo** | Texto cadastrado pelo usuário, associado a um atalho global que o cola. Código: `Snippet`. |
| **Atalho global (hotkey)** | Combinação de teclas registrada no sistema, funciona com qualquer app em foco. Código: `Hotkey`. |
| **Retenção** | Regras de expiração (idade máxima e quantidade máxima). Código: `RetentionPolicy`. |
| **Janela alvo** | Janela que estava em primeiro plano antes de abrir o popup; recebe a colagem. |
| **Captura própria** | Alteração do clipboard feita pelo próprio ClipVault (ao colar); tratada para não duplicar itens. |
