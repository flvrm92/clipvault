# 07 — Identidade visual

> Status: Aprovada · v1.0 · 2026-11-20

## 1. Conceito

**ClipVault = clipboard + cofre.** A marca comunica *guardar com segurança e recuperar rápido*.

- **Personalidade:** confiável, ágil, discreta, moderna.
- **Símbolo (proposta):** uma prancheta (clipboard) estilizada em forma de quadrado arredondado, cujo **clipe superior forma o "trinco" de um cofre**; no corpo, duas ou três linhas horizontais escalonadas sugerem itens empilhados (histórico). Geometria simples, legível a 16 px.
- **Regras do símbolo:** grid de 24 unidades; traço/forma com raio consistente; sem gradientes no tamanho ≤ 32 px (versão "flat"); com gradiente sutil nos tamanhos de tile/Store.

> Entregável do marco M6 ([10](10-roadmap.md)): 3 variações do símbolo em SVG para escolha antes da geração de todos os assets.

## 2. Paleta

### 2.1 Cores da marca

| Token | Hex | Uso |
|-------|-----|-----|
| `brand-600` | `#4B3FD1` | Cor primária (ícone, botões primários no tema claro) |
| `brand-500` | `#6157E8` | Hover / ícone em fundo escuro |
| `brand-400` | `#8A82F2` | Acento no tema escuro |
| `brand-100` | `#ECEBFF` | Fundos de seleção no tema claro |
| `vault-teal` | `#14B8A6` | Cor secundária (detalhe do trinco, estados de sucesso) |
| `ink-900` | `#14131F` | Fundo de tile/hero escuro |

### 2.2 Tokens semânticos

| Token | Claro | Escuro |
|-------|-------|--------|
| `--surface` | `#F7F7FA` (ou Mica) | `#1C1B22` (ou Mica) |
| `--surface-raised` | `#FFFFFF` | `#26252E` |
| `--surface-hover` | `#EEEEF4` | `#302F3A` |
| `--surface-selected` | `#ECEBFF` | `#2E2A5C` |
| `--border` | `#E1E0EA` | `#3A3946` |
| `--text-primary` | `#1B1A24` | `#F2F1F7` |
| `--text-secondary` | `#5C5A6B` | `#A9A7B8` |
| `--accent` | `#4B3FD1` | `#8A82F2` |
| `--accent-text` (sobre accent) | `#FFFFFF` | `#14131F` |
| `--success` | `#0F8A7E` | `#2DD4BF` |
| `--warning` | `#9A6700` | `#F5C451` |
| `--danger` | `#C4314B` | `#FF8A9B` |
| `--focus-ring` | `#4B3FD1` | `#A9A3FF` |

> Todos os pares texto/fundo devem passar WCAG AA (A11Y-002). Um teste automatizado (Vitest) lê os tokens e valida as razões de contraste.

## 3. Tipografia

- **Interface:** `Segoe UI Variable` (fallback `Segoe UI`, `system-ui`) — nativa, sem baixar fontes.
- **Código/prévias técnicas:** `Cascadia Mono` (fallback `Consolas`, monospace).
- Escala (px): 12 legenda · 14 corpo · 16 subtítulo · 20 título de seção · 28 título de página.
- Pesos: 400 corpo, 600 títulos.

## 4. Iconografia de interface

- **Fluent UI System Icons** (licença MIT), estilo *Regular* 20 px; *Filled* para estados ativos (ex.: pin fixado).
- Ícones importados como SVG inline (sem fonte de ícones), cor via `currentColor`.

## 5. Movimento

- Durações: 120 ms (micro), 200 ms (entrada de painel). Curva: `cubic-bezier(0.2, 0, 0, 1)`.
- Sem animações com movimento reduzido (A11Y-004).

## 6. Assets a produzir

Fonte única: SVGs mestres em `assets/brand/` → geração automatizada por script (`tools/gen-assets`, usando `resvg` + `image` ou `ImageMagick`) para todos os tamanhos. **Assets gerados são versionados** e o script é idempotente; CI verifica que estão atualizados.

### 6.1 SVGs mestres

| Arquivo | Descrição |
|---------|-----------|
| `logo-symbol.svg` | Símbolo com detalhes (tiles, Store) |
| `logo-symbol-flat.svg` | Símbolo simplificado para ≤ 32 px |
| `logo-wordmark.svg` | Símbolo + "ClipVault" |
| `tray-light.svg` / `tray-dark.svg` | Silhueta monocromática para barra clara/escura |
| `tray-paused-light.svg` / `tray-paused-dark.svg` | Variante "pausado" (TRAY-003) |
| `empty-state.svg` | Ilustração do estado vazio (usa tokens via `currentColor`) |

### 6.2 Aplicativo / bandeja

| Asset | Tamanhos |
|-------|----------|
| `app.ico` (executável) | 16, 20, 24, 32, 40, 48, 64, 256 |
| `tray-*.ico` (4 variantes) | 16, 20, 24, 32, 40 (cobre 100%–250% DPI) |

### 6.3 Pacote MSIX (`packaging/msix/Assets/`)

Escalas `100, 125, 150, 200, 400` para cada logo:

| Asset do manifesto | Tamanho base (100%) |
|--------------------|---------------------|
| `Square44x44Logo` | 44 × 44 + `targetsize-16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 256` (com variantes `altform-unplated` e `altform-lightunplated`) |
| `SmallTile` (Square71x71Logo) | 71 × 71 |
| `Square150x150Logo` | 150 × 150 |
| `Wide310x150Logo` | 310 × 150 |
| `LargeTile` (Square310x310Logo) | 310 × 310 |
| `StoreLogo` | 50 × 50 |
| `SplashScreen` | 620 × 300 *(não exibido em apps Win32, mas exigido pela validação de alguns perfis; manter)* |

### 6.4 Listagem na Store (Partner Center)

> Conferir dimensões vigentes no Partner Center no momento da submissão.

| Asset | Observação |
|-------|-----------|
| Capturas de tela | Mín. 1, recomendado 4–6, 1920 × 1080 (16:9): popup claro, popup escuro, busca, snippets, configurações |
| Logo da Store | 1:1 (300 × 300 e 2160 × 2160) |
| Poster art 9:16 | 720 × 1080 e 1440 × 2160 |
| Box art 1:1 | 1080 × 1080 e 2160 × 2160 |
| Hero / Super hero 16:9 | 1920 × 1080 e 3840 × 2160 |
| Vídeo de trailer *(opcional)* | 15–30 s mostrando abrir → buscar → colar → snippet |

### 6.5 Checklist de qualidade dos assets

- [ ] Legível a 16 px nas barras clara e escura.
- [ ] Ícones `unplated` sem fundo, sem corte nas bordas (margem de segurança de ~1/12).
- [ ] Contraste do ícone ≥ 3:1 contra fundos claro e escuro.
- [ ] Verificado em 100%, 150% e 200% de escala.
- [ ] Todas as variantes geradas a partir dos SVGs mestres (sem edição manual de PNGs).
