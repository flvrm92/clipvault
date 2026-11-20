-- Migration 001 — initial schema (specs/05-modelo-de-dados.md §2). Never edit after release.

CREATE TABLE clip_items (
    id            INTEGER PRIMARY KEY,
    kind          TEXT    NOT NULL CHECK (kind IN ('text', 'image')),
    text_content  TEXT,
    search_text   TEXT,
    preview       TEXT    NOT NULL,
    content_hash  BLOB    NOT NULL UNIQUE,
    byte_size     INTEGER NOT NULL CHECK (byte_size >= 0),
    source_app    TEXT,
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
    file_key       TEXT    NOT NULL UNIQUE,
    width          INTEGER NOT NULL CHECK (width  > 0),
    height         INTEGER NOT NULL CHECK (height > 0),
    file_size      INTEGER NOT NULL,
    thumb_width    INTEGER NOT NULL,
    thumb_height   INTEGER NOT NULL
);

CREATE TABLE snippets (
    id          INTEGER PRIMARY KEY,
    name        TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    body        TEXT    NOT NULL,
    hotkey      TEXT    UNIQUE,
    enabled     INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE TABLE settings (
    key    TEXT PRIMARY KEY,
    value  TEXT NOT NULL
);
