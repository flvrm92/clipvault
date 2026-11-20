# 06 — UX / UI

> Status: Aprovada · v1.0 · 2026-11-20

## 1. Princípios

1. **Teclado primeiro.** Abrir → buscar → colar sem tocar no mouse.
2. **Rápido e silencioso.** O popup aparece instantaneamente e some sozinho; nenhuma janela "pula" sem pedido.
3. **Nativo do Windows 11.** Cantos arredondados (8 px), Mica/Acrylic, Segoe UI Variable, ícones Fluent — sem parecer "site dentro de janela".
4. **Conteúdo é o protagonista.** Cromo mínimo; prévias legíveis.
5. **Seguro por padrão.** Ações destrutivas pedem confirmação ou oferecem desfazer.

## 2. Popup de histórico

Tamanho padrão **380 × 520 px** (redimensionável, tamanho lembrado). Sem barra de título; arrastável pelo cabeçalho.

```
┌──────────────────────────────────────────┐
│ ◈ ClipVault                    ⚙   ⋯    │  cabeçalho (arrastável)
│ ┌──────────────────────────────────────┐ │
│ │ 🔍 Buscar no histórico…              │ │  foco automático
│ └──────────────────────────────────────┘ │
│  [Tudo] [Texto] [Imagens] [Fixados] [Snippets]
│ ──────────────────────────────────────── │
│ 📌 Atenciosamente,                       │  fixado
│    Flávio Moreno — Equipe X       2 d    │
│ ──────────────────────────────────────── │
│ ▌ git rebase -i HEAD~3            agora  │  ← item focado (barra de destaque)
│ ──────────────────────────────────────── │
│ ┌──────┐                                 │
│ │ img  │  1920 × 1080 · PNG    há 5 min  │
│ └──────┘                                 │
│ ──────────────────────────────────────── │
│ https://github.com/org/repo/pull/42      │
│                                  há 1 h  │
│ ──────────────────────────────────────── │
│ ↵ Colar · ⇧↵ Texto puro · Ctrl+P Fixar · Del Excluir │  rodapé com dicas
└──────────────────────────────────────────┘
```

### 2.1 Linha de item

- Altura fixa de **68 px** para todas as linhas, exigida pela lista virtualizada (HIST-002). O texto ocupa até 3 linhas de 18 px; imagens mostram miniatura de 96 × 56 (`object-fit: cover`, cantos de 4 px).
- A seleção pelo mouse acompanha só **movimento** do ponteiro (`mousemove`). Como o popup abre sob o cursor, `mouseenter` selecionaria a linha que apareceu debaixo de um ponteiro parado, e não o primeiro item.
- Hover/foco: fundo sutil + ações rápidas à direita (📌 fixar, 🗑 excluir) — visíveis só no hover/foco para reduzir ruído.
- Texto em fonte de interface; se detectado como código/linha de comando (heurística simples: começa com `$`, contém `{`/`;` em proporção alta), usar **Cascadia Mono** na prévia — `C`, opcional.
- URLs exibidas com ícone 🔗; cores normais (sem sublinhado).

### 2.2 Teclado no popup

| Tecla | Ação | Req. |
|-------|------|------|
| digitar | busca | HIST-003 |
| `↑` / `↓`, `PgUp` / `PgDn`, `Home` / `End` | navegar | A11Y-001 |
| `Enter` / duplo clique | colar | PST-001 |
| `Shift + Enter` | colar texto normalizado | PST-002 |
| `Ctrl + C` | copiar sem colar | HIST-011 |
| `Alt + 1…9` | colar N-ésimo item | PST-004 |
| `Ctrl + P` | fixar/desafixar | HIST-005 |
| `Delete` | excluir (com "Desfazer" por 5 s em toast) | HIST-006 |
| `Espaço` (com busca vazia ou foco na lista) | pré-visualizar | HIST-008 |
| `Tab` / `Shift + Tab` | alternar filtros | HIST-004 |
| `Ctrl + ,` | abrir configurações | CFG-001 |
| `Esc` | limpar busca; se vazia, fechar | PST-007 |

### 2.3 Comportamento da janela

- Pré-criada e oculta na inicialização (PERF-001).
- Abre perto do caret/mouse (HK-005), com animação de entrada 120 ms (fade + 4 px slide), desativada com movimento reduzido.
- Aberto pela bandeja, a janela em foco é a própria barra de tarefas. Nesse caso, a janela alvo da colagem é a primeira janela de aplicativo visível na ordem de empilhamento (z-order), que é aquela em que o usuário estava trabalhando.
- Menu "⋯" do cabeçalho: Pausar/Retomar captura e Limpar histórico (com confirmação). "Configurações" entra no M6.
- Fecha ao perder foco, `Esc` ou após colar.
- Ao reabrir: busca limpa, filtro "Tudo", foco no 1º item.

### 2.4 Estados

| Estado | Conteúdo |
|--------|----------|
| Vazio | Ilustração da marca + "Nada por aqui ainda" + "Copie algo para começar" + atalho atual |
| Sem resultados | "Nenhum resultado para "xyz"" |
| Captura pausada | Faixa amarela no topo: "Captura pausada · Retomar" |
| Erro de colagem | Toast: "Conteúdo copiado. Cole com Ctrl+V." (PST-005) |

## 3. Janela de configurações

**760 × 560 px**, Mica, navegação lateral (padrão das Configurações do Windows 11).

```
┌────────────────────────────────────────────────────────────┐
│ ◈ ClipVault — Configurações                     ─  □  ✕   │
├──────────────┬─────────────────────────────────────────────┤
│  ⚙ Geral     │  Atalhos                                    │
│  🕘 Histórico │                                             │
│ ▌⌨ Atalhos   │  Abrir o ClipVault                          │
│  🛡 Privacidade│  ┌───────────────────────┐                 │
│  🎨 Aparência │  │  Ctrl + Win + V       │  Alterar        │
│  ℹ Sobre     │  └───────────────────────┘                 │
│              │                                             │
│              │  Conteúdos fixos                 + Novo     │
│              │  ┌─────────────────────────────────────────┐│
│              │  │ Assinatura     Ctrl+Shift+1   ● Ativo  ⋯││
│              │  │ Teste          Ctrl+Shift+V   ● Ativo  ⋯││
│              │  │ Endereço       —              ○ Inativo⋯││
│              │  └─────────────────────────────────────────┘│
│              │  ☑ Restaurar área de transferência após     │
│              │    colar um conteúdo fixo                   │
└──────────────┴─────────────────────────────────────────────┘
```

### Seções

| Seção | Controles |
|-------|-----------|
| **Geral** | Iniciar com o Windows · Idioma · Captura ativa (toggle) |
| **Histórico** | Manter itens por (select) · Quantidade máxima (select) · Limpar ao sair · Tamanho máx. de texto/imagem · Botão "Limpar histórico" |
| **Atalhos** | Atalho de abertura (gravador) · Lista de conteúdos fixos (CRUD) · Restaurar clipboard |
| **Privacidade** | Explicação do que é respeitado (gerenciadores de senha) · Apps excluídos (lista + adicionar por nome de executável ou escolhendo processo em execução) |
| **Aparência** | Tema: Sistema/Claro/Escuro (cards com pré-visualização) · Usar cor de destaque do Windows |
| **Sobre** | Logo, versão, política de privacidade, licenças, suporte, abrir pasta de dados |

### Editor de conteúdo fixo (diálogo)

```
┌─ Novo conteúdo fixo ──────────────────────────┐
│ Nome       [ Assinatura                    ]  │
│ Conteúdo   ┌────────────────────────────────┐ │
│            │ Atenciosamente,                │ │
│            │ Flávio                         │ │
│            └────────────────────────────────┘ │
│ Atalho     [ Pressione as teclas…        ] ✕  │
│            ⚠ Atalho já usado por "Teste"     │  validação inline
│ ☑ Ativo                                       │
│                         [Cancelar] [Salvar]   │
└───────────────────────────────────────────────┘
```

### Gravador de atalho (componente)

- Estados: *ocioso* (mostra atalho em "keycaps"), *gravando* (borda em cor da marca + "Pressione as teclas…"), *inválido* (borda de erro + mensagem), *salvo*.
- Enquanto grava, o backend **suspende temporariamente os atalhos globais do app** para que a combinação chegue ao campo.
- Exibe teclas como keycaps: `[Ctrl] [Win] [V]`.

## 4. Tray

- Tooltip: "ClipVault — 128 itens" / "ClipVault — captura pausada".
- Menu nativo (segue o tema do sistema).

## 5. Temas

- Tokens semânticos (`--surface`, `--surface-raised`, `--text-primary`, `--accent`, …) definidos em [07](07-identidade-visual.md); componentes nunca usam cores literais.
- `data-theme="light|dark"` no `<html>`, definido pelo backend (THM-001) — evita "flash" de tema errado: o tema é injetado antes do primeiro paint.
- Alto contraste: `@media (forced-colors: active)` usa cores do sistema.

## 6. i18n

- Chaves hierárquicas (`popup.search.placeholder`, `settings.history.retention.label`).
- Pluralização via `Intl.PluralRules`; tempos relativos via `Intl.RelativeTimeFormat`.
- Textos de UI curtos o suficiente para EN e PT-BR (PT costuma ser ~20% mais longo — layout deve acomodar).
- Strings do tray/notificações nativas vêm do backend (mesmos arquivos JSON, embutidos no binário).

## 7. Microtextos (exemplos PT-BR / EN)

| Chave | PT-BR | EN |
|-------|-------|----|
| `popup.search.placeholder` | Buscar no histórico… | Search history… |
| `popup.empty.title` | Nada por aqui ainda | Nothing here yet |
| `popup.empty.body` | Copie algo para começar | Copy something to get started |
| `toast.copiedOnly` | Conteúdo copiado. Cole com Ctrl+V. | Copied. Paste with Ctrl+V. |
| `hotkey.conflict.app` | Este atalho está em uso por outro aplicativo | This shortcut is used by another app |
| `history.clear.confirm` | Remover {count} itens? Itens fixados serão mantidos. | Remove {count} items? Pinned items will be kept. |
