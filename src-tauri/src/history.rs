//! 세션 기록: SQLite(+FTS5) 저장·검색·내보내기.
use babelay_engine::engine::EngineEvent;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS sessions(id INTEGER PRIMARY KEY, started_at INTEGER NOT NULL, ended_at INTEGER, src_lang TEXT, tgt_lang TEXT, asr_model TEXT, translator TEXT);
CREATE TABLE IF NOT EXISTS segments(id INTEGER PRIMARY KEY, session_id INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE, t0_ms INTEGER, t1_ms INTEGER, lang TEXT, src_text TEXT NOT NULL, tgt_text TEXT);
CREATE VIRTUAL TABLE IF NOT EXISTS segments_fts USING fts5(src_text, tgt_text, content='segments', content_rowid='id');
CREATE TRIGGER IF NOT EXISTS segments_ai AFTER INSERT ON segments BEGIN INSERT INTO segments_fts(rowid, src_text, tgt_text) VALUES (new.id, new.src_text, new.tgt_text); END;
CREATE TRIGGER IF NOT EXISTS segments_ad AFTER DELETE ON segments BEGIN INSERT INTO segments_fts(segments_fts, rowid, src_text, tgt_text) VALUES('delete', old.id, old.src_text, old.tgt_text); END;
PRAGMA foreign_keys = ON;
";

pub struct Db(pub Mutex<Connection>);

#[derive(Serialize)]
pub struct SessionSummary {
    pub id: i64,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub src_lang: String,
    pub tgt_lang: String,
    pub asr_model: String,
    pub segments: i64,
}

#[derive(Serialize)]
pub struct SegmentRow {
    pub id: i64,
    pub session_id: i64,
    pub t0_ms: i64,
    pub t1_ms: i64,
    pub lang: String,
    pub src_text: String,
    pub tgt_text: Option<String>,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `HH:MM:SS,mmm`
fn srt_time(ms: i64) -> String {
    let ms = ms.max(0);
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1_000 % 60,
        ms % 1_000
    )
}

pub fn open(path: &Path) -> rusqlite::Result<Db> {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let conn = Connection::open(path)?;
    conn.execute_batch(SCHEMA)?;
    Ok(Db(Mutex::new(conn)))
}

#[cfg(test)]
pub fn open_in_memory() -> rusqlite::Result<Db> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(SCHEMA)?;
    Ok(Db(Mutex::new(conn)))
}

impl Db {
    /// 잠금이 오염돼도 기록 때문에 캡처가 죽지 않게 한다.
    fn conn(&self) -> MutexGuard<'_, Connection> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn begin_session(
        &self,
        src_lang: &str,
        tgt_lang: &str,
        asr_model: &str,
    ) -> rusqlite::Result<i64> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO sessions(started_at, src_lang, tgt_lang, asr_model) VALUES(?1, ?2, ?3, ?4)",
            params![now(), src_lang, tgt_lang, asr_model],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn end_session(&self, id: i64) -> rusqlite::Result<()> {
        self.conn().execute(
            "UPDATE sessions SET ended_at = ?1 WHERE id = ?2",
            params![now(), id],
        )?;
        Ok(())
    }

    pub fn insert_segment(
        &self,
        session_id: i64,
        t0_ms: i64,
        t1_ms: i64,
        lang: &str,
        src_text: &str,
    ) -> rusqlite::Result<()> {
        self.conn().execute(
            "INSERT INTO segments(session_id, t0_ms, t1_ms, lang, src_text) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![session_id, t0_ms, t1_ms, lang, src_text],
        )?;
        Ok(())
    }

    pub fn sessions(&self, limit: u32) -> rusqlite::Result<Vec<SessionSummary>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT s.id, s.started_at, s.ended_at, s.src_lang, s.tgt_lang, s.asr_model, COUNT(g.id)
             FROM sessions s LEFT JOIN segments g ON g.session_id = s.id
             GROUP BY s.id ORDER BY s.started_at DESC, s.id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], |r| {
            Ok(SessionSummary {
                id: r.get(0)?,
                started_at: r.get(1)?,
                ended_at: r.get(2)?,
                src_lang: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                tgt_lang: r.get::<_, Option<String>>(4)?.unwrap_or_default(),
                asr_model: r.get::<_, Option<String>>(5)?.unwrap_or_default(),
                segments: r.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn segments(&self, session_id: i64) -> rusqlite::Result<Vec<SegmentRow>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, t0_ms, t1_ms, lang, src_text, tgt_text FROM segments
             WHERE session_id = ?1 ORDER BY t0_ms, id",
        )?;
        let rows = stmt.query_map([session_id], segment_row)?;
        rows.collect()
    }

    /// FTS5 문법이 새어 들어가지 않게 통째로 구(phrase)로 감싼다.
    pub fn search(&self, q: &str) -> rusqlite::Result<Vec<SegmentRow>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT g.id, g.session_id, g.t0_ms, g.t1_ms, g.lang, g.src_text, g.tgt_text
             FROM segments_fts f JOIN segments g ON g.id = f.rowid
             WHERE segments_fts MATCH ?1 ORDER BY rank LIMIT 200",
        )?;
        let rows = stmt.query_map([format!("\"{}\"", q.replace('"', ""))], segment_row)?;
        rows.collect()
    }

    /// segments 는 FK 로 함께 지워지고, 삭제 트리거가 FTS 인덱스를 정리한다.
    pub fn delete_session(&self, id: i64) -> rusqlite::Result<()> {
        self.conn()
            .execute("DELETE FROM sessions WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn export(&self, session_id: i64, fmt: &str) -> rusqlite::Result<String> {
        let rows = self.segments(session_id)?;
        if fmt == "srt" {
            return Ok(rows
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    format!(
                        "{}\n{} --> {}\n{}\n\n",
                        i + 1,
                        srt_time(r.t0_ms),
                        srt_time(r.t1_ms),
                        r.src_text
                    )
                })
                .collect());
        }
        Ok(rows.iter().map(|r| format!("{}\n", r.src_text)).collect())
    }
}

fn segment_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<SegmentRow> {
    Ok(SegmentRow {
        id: r.get(0)?,
        session_id: r.get(1)?,
        t0_ms: r.get::<_, Option<i64>>(2)?.unwrap_or(0),
        t1_ms: r.get::<_, Option<i64>>(3)?.unwrap_or(0),
        lang: r.get::<_, Option<String>>(4)?.unwrap_or_default(),
        src_text: r.get(5)?,
        tgt_text: r.get(6)?,
    })
}

/// 현재 세션 id 슬롯. DB 가 없으면(열기 실패) 기록은 통째로 비활성이다.
fn current(app: &AppHandle) -> MutexGuard<'_, Option<i64>> {
    let state = app.state::<crate::session::SessionState>().inner();
    state.session_id.lock().unwrap_or_else(|p| p.into_inner())
}

pub fn begin(app: &AppHandle, src_lang: &str, tgt_lang: &str, asr_model: &str) {
    let Some(db) = app.try_state::<Db>() else {
        return;
    };
    match db.begin_session(src_lang, tgt_lang, asr_model) {
        Ok(id) => *current(app) = Some(id),
        Err(e) => eprintln!("history: begin_session failed: {e}"),
    }
}

pub fn end(app: &AppHandle) {
    let Some(id) = current(app).take() else {
        return;
    };
    if let Some(db) = app.try_state::<Db>() {
        if let Err(e) = db.end_session(id) {
            eprintln!("history: end_session failed: {e}");
        }
    }
}

/// 중계 스레드에서 불린다 — 어떤 실패도 로그로만 남기고 삼킨다.
pub fn on_final(app: &AppHandle, ev: &EngineEvent) {
    let EngineEvent::Final {
        text,
        lang,
        start_ms,
        end_ms,
        ..
    } = ev
    else {
        return;
    };
    let Some(id) = *current(app) else { return };
    let Some(db) = app.try_state::<Db>() else {
        return;
    };
    if let Err(e) = db.insert_segment(id, *start_ms as i64, *end_ms as i64, lang, text) {
        eprintln!("history: insert_segment failed: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn insert_search_and_export() {
        let db = open_in_memory().unwrap();
        let sid = db.begin_session("en", "ko", "small").unwrap();
        db.insert_segment(sid, 0, 1200, "en", "hello world")
            .unwrap();
        db.insert_segment(sid, 1200, 2500, "en", "second line")
            .unwrap();
        db.end_session(sid).unwrap();
        assert_eq!(db.sessions(10).unwrap()[0].segments, 2);
        assert_eq!(db.search("world").unwrap().len(), 1);
        let srt = db.export(sid, "srt").unwrap();
        assert!(srt.starts_with("1\n00:00:00,000 --> 00:00:01,200\nhello world\n\n2\n"));
        let txt = db.export(sid, "txt").unwrap();
        assert_eq!(txt, "hello world\nsecond line\n");
        db.delete_session(sid).unwrap();
        assert!(db.sessions(10).unwrap().is_empty());
        assert!(db.search("world").unwrap().is_empty());
    }
}
