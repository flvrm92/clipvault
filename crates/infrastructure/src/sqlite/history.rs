//! [`HistoryRepository`] over SQLite (specs/05 §2).

use clipvault_application::ports::{
    ClearScope, HistoryQuery, HistoryRepository, KindFilter, Page, PortError, Removal,
};
use clipvault_domain::app_name::AppName;
use clipvault_domain::clip::{ClipContent, ClipItem, NewClipItem, PinState};
use clipvault_domain::hash::ContentHash;
use clipvault_domain::ids::ClipId;
use clipvault_domain::image::{FileKey, ImageMeta};
use clipvault_domain::retention::RetentionCandidate;
use clipvault_domain::text::{ClipText, normalize_for_search};
use clipvault_domain::time::Timestamp;
use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, Transaction, params, params_from_iter};

use super::Database;

const SELECT_ITEM: &str = "
    SELECT c.id, c.text_content, c.content_hash, c.source_app, c.pinned, c.use_count,
           c.created_at, c.last_used_at,
           a.file_key, a.width, a.height, a.file_size, a.thumb_width, a.thumb_height
    FROM clip_items c
    LEFT JOIN image_assets a ON a.item_id = c.id";

const ORDER: &str = "ORDER BY c.pinned DESC, c.last_used_at DESC, c.id DESC";

/// History items stored in SQLite.
#[derive(Debug, Clone)]
pub struct SqliteHistoryRepository {
    db: Database,
}

impl SqliteHistoryRepository {
    /// A repository over `db`.
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

fn conversion_error(column: usize, message: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(column, Type::Text, message.to_owned().into())
}

fn u32_column(row: &Row<'_>, column: usize) -> rusqlite::Result<u32> {
    u32::try_from(row.get::<_, i64>(column)?)
        .map_err(|_| conversion_error(column, "value out of range"))
}

fn read_item(row: &Row<'_>) -> rusqlite::Result<ClipItem> {
    let hash_bytes: Vec<u8> = row.get(2)?;
    let hash = <[u8; 32]>::try_from(hash_bytes.as_slice())
        .map(ContentHash::from_bytes)
        .map_err(|_| conversion_error(2, "invalid content hash"))?;

    let content = if let Some(text) = row.get::<_, Option<String>>(1)? {
        ClipContent::Text(ClipText::new(&text).map_err(|_| conversion_error(1, "blank text item"))?)
    } else {
        let key: String = row.get(8)?;
        ClipContent::Image(ImageMeta {
            file_key: FileKey::parse(&key).map_err(|_| conversion_error(8, "invalid file key"))?,
            width: u32_column(row, 9)?,
            height: u32_column(row, 10)?,
            file_size: u64::try_from(row.get::<_, i64>(11)?)
                .map_err(|_| conversion_error(11, "negative file size"))?,
            thumb_width: u32_column(row, 12)?,
            thumb_height: u32_column(row, 13)?,
        })
    };

    Ok(ClipItem {
        id: ClipId::new(row.get(0)?),
        content,
        hash,
        source_app: row
            .get::<_, Option<String>>(3)?
            .and_then(|name| AppName::parse(&name).ok()),
        pinned: row.get(4)?,
        use_count: u32_column(row, 5)?,
        created_at: Timestamp::from_millis(row.get(6)?),
        last_used_at: Timestamp::from_millis(row.get(7)?),
    })
}

/// Escapes `%`, `_` and the escape character itself for a `LIKE … ESCAPE '\'` pattern.
fn like_pattern(term: &str) -> String {
    let escaped = term
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{}%", normalize_for_search(&escaped))
}

fn not_found_if_unchanged(changed: usize) -> Result<(), PortError> {
    if changed == 0 {
        Err(PortError::NotFound)
    } else {
        Ok(())
    }
}

fn image_key_of(conn: &Connection, id: ClipId) -> rusqlite::Result<Option<FileKey>> {
    conn.query_row(
        "SELECT file_key FROM image_assets WHERE item_id = ?1",
        [id.get()],
        |row| row.get::<_, String>(0),
    )
    .optional()?
    .map(|key| FileKey::parse(&key).map_err(|_| conversion_error(0, "invalid file key")))
    .transpose()
}

/// Deletes the items matching `condition` (a SQL predicate over `clip_items` aliased `c`).
fn delete_where(
    tx: &Transaction<'_>,
    condition: &str,
    params: &[i64],
) -> rusqlite::Result<Removal> {
    let mut keys = tx.prepare(&format!(
        "SELECT a.file_key FROM image_assets a JOIN clip_items c ON c.id = a.item_id WHERE {condition}"
    ))?;
    let image_keys = keys
        .query_map(params_from_iter(params), |row| row.get::<_, String>(0))?
        .map(|key| {
            key.and_then(|k| {
                FileKey::parse(&k).map_err(|_| conversion_error(0, "invalid file key"))
            })
        })
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let count = tx.execute(
        &format!("DELETE FROM clip_items AS c WHERE {condition}"),
        params_from_iter(params),
    )?;
    Ok(Removal { count, image_keys })
}

impl HistoryRepository for SqliteHistoryRepository {
    fn find_by_hash(&self, hash: &ContentHash) -> Result<Option<ClipId>, PortError> {
        self.db.with(|conn| {
            conn.query_row(
                "SELECT id FROM clip_items WHERE content_hash = ?1",
                [hash.as_bytes().as_slice()],
                |row| row.get(0).map(ClipId::new),
            )
            .optional()
        })
    }

    fn insert(&self, item: NewClipItem) -> Result<ClipId, PortError> {
        self.db.with(|conn| {
            let tx = conn.transaction()?;
            let (kind, text, search, preview) = match &item.content {
                ClipContent::Text(text) => (
                    "text",
                    Some(text.as_str()),
                    Some(text.search_form()),
                    text.preview(),
                ),
                ClipContent::Image(_) => ("image", None, None, String::new()),
            };
            tx.execute(
                "INSERT INTO clip_items (kind, text_content, search_text, preview, content_hash,
                     byte_size, source_app, pinned, use_count, created_at, last_used_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 1, ?8, ?8)",
                params![
                    kind,
                    text,
                    search,
                    preview,
                    item.hash.as_bytes().as_slice(),
                    i64::try_from(item.content.byte_size()).unwrap_or(i64::MAX),
                    item.source_app.as_ref().map(AppName::as_str),
                    item.captured_at.as_millis(),
                ],
            )?;
            let id = tx.last_insert_rowid();
            if let ClipContent::Image(meta) = &item.content {
                tx.execute(
                    "INSERT INTO image_assets (item_id, file_key, width, height, file_size,
                         thumb_width, thumb_height)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        id,
                        meta.file_key.as_str(),
                        meta.width,
                        meta.height,
                        i64::try_from(meta.file_size).unwrap_or(i64::MAX),
                        meta.thumb_width,
                        meta.thumb_height,
                    ],
                )?;
            }
            tx.commit()?;
            Ok(ClipId::new(id))
        })
    }

    fn get(&self, id: ClipId) -> Result<Option<ClipItem>, PortError> {
        self.db.with(|conn| {
            conn.query_row(
                &format!("{SELECT_ITEM} WHERE c.id = ?1"),
                [id.get()],
                read_item,
            )
            .optional()
        })
    }

    fn list(&self, query: &HistoryQuery) -> Result<Page<ClipItem>, PortError> {
        let mut conditions = vec![match query.filter {
            KindFilter::All => "1 = 1",
            KindFilter::Text => "c.kind = 'text'",
            KindFilter::Images => "c.kind = 'image'",
            KindFilter::Pinned => "c.pinned = 1",
        }];
        let mut args: Vec<rusqlite::types::Value> = Vec::new();
        if let Some(term) = &query.search {
            conditions.push("c.search_text LIKE ? ESCAPE '\\'");
            args.push(like_pattern(term).into());
        }
        let limit = i64::try_from(query.limit).unwrap_or(i64::MAX);
        args.push(limit.saturating_add(1).into());
        args.push(i64::try_from(query.offset).unwrap_or(i64::MAX).into());

        let sql = format!(
            "{SELECT_ITEM} WHERE {} {ORDER} LIMIT ? OFFSET ?",
            conditions.join(" AND ")
        );
        let mut items = self.db.with(|conn| {
            conn.prepare(&sql)?
                .query_map(params_from_iter(args), read_item)?
                .collect::<rusqlite::Result<Vec<_>>>()
        })?;
        let has_more = items.len() > query.limit;
        items.truncate(query.limit);
        Ok(Page { items, has_more })
    }

    fn touch(&self, id: ClipId, at: Timestamp) -> Result<(), PortError> {
        let changed = self.db.with(|conn| {
            conn.execute(
                "UPDATE clip_items SET last_used_at = ?2, use_count = use_count + 1 WHERE id = ?1",
                params![id.get(), at.as_millis()],
            )
        })?;
        not_found_if_unchanged(changed)
    }

    fn set_pin_state(&self, id: ClipId, state: PinState) -> Result<(), PortError> {
        let changed = self.db.with(|conn| {
            conn.execute(
                "UPDATE clip_items SET pinned = ?2 WHERE id = ?1",
                params![id.get(), state.is_pinned()],
            )
        })?;
        not_found_if_unchanged(changed)
    }

    fn delete(&self, id: ClipId) -> Result<Option<FileKey>, PortError> {
        let (key, changed) = self.db.with(|conn| {
            let tx = conn.transaction()?;
            let key = image_key_of(&tx, id)?;
            let changed = tx.execute("DELETE FROM clip_items WHERE id = ?1", [id.get()])?;
            tx.commit()?;
            Ok((key, changed))
        })?;
        not_found_if_unchanged(changed).map(|()| key)
    }

    fn delete_many(&self, ids: &[ClipId]) -> Result<Removal, PortError> {
        if ids.is_empty() {
            return Ok(Removal::default());
        }
        self.db.with(|conn| {
            let tx = conn.transaction()?;
            let mut total = Removal::default();
            // Bounded batches keep the statement under SQLite's parameter limit.
            for batch in ids.chunks(500) {
                let placeholders = vec!["?"; batch.len()].join(", ");
                let raw: Vec<i64> = batch.iter().map(|id| id.get()).collect();
                let removal = delete_where(&tx, &format!("c.id IN ({placeholders})"), &raw)?;
                total.count += removal.count;
                total.image_keys.extend(removal.image_keys);
            }
            tx.commit()?;
            Ok(total)
        })
    }

    fn clear(&self, scope: ClearScope) -> Result<Removal, PortError> {
        let condition = match scope {
            ClearScope::UnpinnedOnly => "c.pinned = 0",
            ClearScope::Everything => "1 = 1",
        };
        self.db.with(|conn| {
            let tx = conn.transaction()?;
            let removal = delete_where(&tx, condition, &[])?;
            tx.commit()?;
            Ok(removal)
        })
    }

    fn retention_candidates(&self) -> Result<Vec<RetentionCandidate>, PortError> {
        self.db.with(|conn| {
            conn.prepare("SELECT id, last_used_at, pinned FROM clip_items")?
                .query_map([], |row| {
                    Ok(RetentionCandidate {
                        id: ClipId::new(row.get(0)?),
                        last_used_at: Timestamp::from_millis(row.get(1)?),
                        pinned: row.get(2)?,
                    })
                })?
                .collect()
        })
    }

    fn image_entries(&self) -> Result<Vec<(ClipId, FileKey)>, PortError> {
        self.db.with(|conn| {
            conn.prepare("SELECT item_id, file_key FROM image_assets ORDER BY item_id")?
                .query_map([], |row| {
                    let key: String = row.get(1)?;
                    let key = FileKey::parse(&key)
                        .map_err(|_| conversion_error(1, "invalid file key"))?;
                    Ok((ClipId::new(row.get(0)?), key))
                })?
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_patterns_escape_wildcards_and_normalize_the_term() {
        assert_eq!(like_pattern("Café"), "%cafe%");
        assert_eq!(like_pattern("100%"), "%100\\%%");
        assert_eq!(like_pattern("a_b"), "%a\\_b%");
        assert_eq!(like_pattern(r"c:\x"), r"%c:\\x%");
    }
}
