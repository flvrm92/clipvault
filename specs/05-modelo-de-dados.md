# 05 — Modelo de dados

> Status: Aprovada · v1.0 · 2026-11-20

## 1. Princípios

- **SQLite** guarda itens, metadados de imagem, snippets e configurações.
- **Imagens ficam em disco** (PNG) na pasta de dados do app; o banco guarda só metadados e a chave do arquivo.
- Datas em **Unix epoch milissegundos UTC** (`INTEGER`).
- `PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA synchronous=NORMAL;`
- Migrações versionadas (`rusqlite_migration`, `PRAGMA user_version`), apenas para frente.

## 2. Esquema (migração 001)

```sql
CREATE TABLE clip_items (
    id            INTEGER PRIMARY KEY,
    kind          TEXT    NOT NULL CHECK (kind IN ('text', 'image')),
    text_content  TEXT,                         -- NULL para imagens
    search_text   TEXT,                         -- texto normalizado (minúsculo, sem acentos) p/ HIST-003
    preview       TEXT    NOT NULL,             -- prévia pronta p/ listagem (≤ 300 chars)
    content_hash  BLOB    NOT NULL UNIQUE,      -- SHA-256 (32 bytes)
    byte_size     INTEGER NOT NULL CHECK (byte_size >= 0),
    source_app    TEXT,                         -- ex.: 'code.exe' (CAP-011)
    pinned        INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    use_count     INTEGER NOT NULL DEFAULT 1,
    created_at    INTEGER NOT NULL,
    last_used_at  INTEGER NOT NULL,
    CHECK ((kind = 'text'  AND text_content IS NOT NULL) OR
           (kind = 'image' AND text_content IS NULL))
);

CREATE INDEX ix_clip_items_order ON clip_items (pinned DESC, last_used_at DESC);
CREATE INDEX ix_clip_items_last_used ON clip_items (last_used_at);

CREATE TABLE image_assets (
    item_id        INTEGER PRIMARY KEY REFERENCES clip_items(id) ON DELETE CASCADE,
    file_key       TEXT    NOT NULL UNIQUE,     -- hex do hash; nome do arquivo
    width          INTEGER NOT NULL CHECK (width  > 0),
    height         INTEGER NOT NULL CHECK (height > 0),
    file_size      INTEGER NOT NULL,            -- bytes do PNG em disco
    thumb_width    INTEGER NOT NULL,
    thumb_height   INTEGER NOT NULL
);

CREATE TABLE snippets (
    id          INTEGER PRIMARY KEY,
    name        TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    body        TEXT    NOT NULL,
    hotkey      TEXT    UNIQUE,                 -- forma canônica: 'Ctrl+Shift+1'; NULL = sem atalho
    enabled     INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE TABLE settings (
    key    TEXT PRIMARY KEY,
    value  TEXT NOT NULL                        -- JSON
);
```

### Observações

- **Busca (HIST-003):** `search_text LIKE '%' || ?1 || '%'` sobre a coluna normalizada atende PERF-004 para 5.000 itens. Se benchmarks mostrarem o contrário, migrar para FTS5 (tabela virtual `clip_items_fts` com tokenizer `unicode61 remove_diacritics 2`) numa migração futura — decisão registrada como ponto de reavaliação no ADR-0003.
- **Deduplicação (CAP-004):** `content_hash UNIQUE` + `INSERT ... ON CONFLICT(content_hash) DO UPDATE SET last_used_at = excluded.last_used_at, use_count = use_count + 1`.
- **Hash:** texto → SHA-256 dos bytes UTF-8 *sem* normalização (dois textos que diferem em espaços são itens diferentes); imagem → SHA-256 dos pixels RGBA decodificados + dimensões (a mesma imagem em `DIB` ou `PNG` gera o mesmo hash).
- **Hotkey única no banco** evita conflitos persistidos; a validação de conflito com o atalho de abertura (que vive em `settings`) é feita no caso de uso.

## 3. Configurações (`settings`)

Gravadas como pares chave/JSON; lidas e convertidas para o agregado `Settings`, com fallback para o padrão por chave (CFG-006).

| Chave | Tipo | Padrão |
|-------|------|--------|
| `capture.paused` | bool | `false` |
| `capture.max_text_bytes` | int | `1048576` |
| `capture.max_image_bytes` | int | `26214400` |
| `capture.excluded_apps` | string[] | `[]` |
| `retention.max_age_days` | int \| null | `30` |
| `retention.max_items` | int \| null | `1000` |
| `retention.clear_on_exit` | bool | `false` |
| `hotkeys.open` | string | `"Ctrl+Win+V"` |
| `snippets.restore_clipboard` | bool | `true` |
| `snippets.restore_delay_ms` | int | `500` |
| `ui.theme` | `"system"\|"light"\|"dark"` | `"system"` |
| `ui.language` | `"system"\|"pt-BR"\|"en"` | `"system"` |
| `ui.use_accent_color` | bool | `false` |
| `general.launch_at_startup` | bool | *(lido do StartupTask; não persistido aqui)* |
| `general.first_run_done` | bool | `false` |
| `schema.settings_version` | int | `1` |

## 4. Armazenamento de imagens

```
<data>/images/
  └─ 3f/
     ├─ 3fa9…c1.png         # original, PNG (compressão padrão)
     └─ 3fa9…c1.thumb.png   # miniatura: lado maior = 320 px, proporção mantida
```

- `file_key` = hex do `content_hash` → nomes determinísticos, dedupe natural.
- **Gravação (REL-002):** encode → `*.tmp` → `fsync` → `rename` → `INSERT` em transação. Se o `INSERT` falhar, os arquivos são removidos.
- **Exclusão:** `DELETE` no banco (transação) → remover arquivos. Falha ao remover arquivo é logada; a reconciliação (RET-005) limpa depois.
- **Reconciliação na inicialização:** (a) arquivos sem `file_key` no banco → removidos; (b) `image_assets` sem arquivo original → item excluído.
- Miniaturas são servidas à UI via `clip://thumb/<item_id>`; originais via `clip://image/<item_id>` (SEC-003).

## 5. Retenção — algoritmo

Função pura em `domain::RetentionPolicy`:

```
expirados = { i ∈ itens | ¬i.pinned ∧ max_age ≠ ∅ ∧ now − i.last_used_at > max_age }
restantes = (itens não fixados) − expirados, ordenados por last_used_at desc
excedentes = restantes[max_items..]   (se max_items ≠ ∅)
remover = expirados ∪ excedentes
```

Na prática o repositório executa equivalentes em SQL (`DELETE … WHERE pinned = 0 AND last_used_at < ?` e `… NOT IN (SELECT id … ORDER BY last_used_at DESC LIMIT ?)`), e **os dois caminhos são testados contra os mesmos casos** (a função pura é o oráculo do teste de integração).

## 6. Migrações

- Diretório `crates/infrastructure/src/sqlite/migrations/NNN_descricao.sql`.
- Teste de migração: cria banco na versão N (fixture), aplica até a última, valida esquema e dados.
- Nunca editar uma migração publicada.
