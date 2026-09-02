import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { PillButton } from "../../components/PillButton";
import { clock } from "../../lib/session";
import { useSettings } from "../../lib/settings";
import { api } from "../../lib/tauri";
import type { SegmentRow, SessionSummary } from "../../lib/types";

export default function History() {
  const { t } = useTranslation();
  const setError = useSettings((s) => s.setError);
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [sel, setSel] = useState<number | null>(null);
  const [segments, setSegments] = useState<SegmentRow[]>([]);
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<SegmentRow[] | null>(null);
  const [saved, setSaved] = useState<string | null>(null);

  // history 커맨드는 DB 상태가 없으면 "state not managed"로 거절한다. 원문 대신 사람 말로.
  const fail = (e: unknown) => {
    const msg = e instanceof Error ? e.message : String(e);
    setError(msg.includes("state not managed") ? t("errors.historyUnavailable") : msg);
  };

  const load = () => api.historySessions(100).then(setSessions).catch(fail);
  useEffect(() => { load(); }, []);
  useEffect(() => { if (sel !== null) api.historySegments(sel).then(setSegments).catch(fail); }, [sel]);

  // 타이핑마다 쿼리를 던지면 DB가 앓는다.
  useEffect(() => {
    const term = q.trim();
    if (!term) { setHits(null); return; }
    const id = window.setTimeout(() => { api.historySearch(term).then(setHits).catch(fail); }, 300);
    return () => window.clearTimeout(id);
  }, [q]);

  const toast = (path: string) => { setSaved(path); window.setTimeout(() => setSaved(null), 2000); };
  const exportAs = (id: number, format: "txt" | "srt") => api.historyExport(id, format).then(toast).catch(fail);
  const remove = (id: number) => api.historyDelete(id).then(() => { setSel(null); load(); }).catch(fail);

  const when = (epoch: number) => new Date(epoch * 1000).toLocaleString();
  const duration = (s: SessionSummary) => (s.ended_at ? clock((s.ended_at - s.started_at) * 1000) : "—");
  const current = sessions.find((s) => s.id === sel);

  return (
    <div className="flex max-w-3xl flex-col gap-4">
      <div className="flex items-center justify-between gap-4">
        <h2 className="text-2xl font-bold">{t("nav.history")}</h2>
        <input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder={t("history.search")}
          aria-label={t("history.search")}
          className="w-56 rounded-full bg-surface px-3 py-1.5 text-sm text-fg placeholder:text-fg-muted"
        />
      </div>

      {saved && <div className="rounded-md bg-surface-2 px-3 py-2 text-xs text-fg-muted">{t("history.saved", { path: saved })}</div>}

      {hits ? (
        <div className="flex flex-col gap-1">
          {hits.map((r) => (
            <button key={r.id} type="button" onClick={() => { setQ(""); setSel(r.session_id); }} className="flex gap-3 rounded-md px-3 py-2 text-left text-sm hover:bg-surface">
              <span className="shrink-0 tabular-nums text-fg-muted">{clock(r.t0_ms)}</span>
              <span className="min-w-0 break-words">{r.src_text}</span>
            </button>
          ))}
        </div>
      ) : sel !== null && current ? (
        <div className="flex flex-col gap-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <PillButton variant="ghost" onClick={() => setSel(null)}>{`← ${t("nav.history")}`}</PillButton>
            <div className="flex gap-2">
              <PillButton onClick={() => exportAs(current.id, "txt")}>{t("history.exportTxt")}</PillButton>
              <PillButton onClick={() => exportAs(current.id, "srt")}>{t("history.exportSrt")}</PillButton>
              <PillButton variant="outline" onClick={() => remove(current.id)}>{t("history.delete")}</PillButton>
            </div>
          </div>
          <div className="text-xs text-fg-muted">
            {when(current.started_at)} · {duration(current)} · {current.src_lang.toUpperCase()} → {current.tgt_lang.toUpperCase()} · {t("history.segments", { count: current.segments })}
          </div>
          <div className="flex flex-col gap-2 rounded-[var(--radius-card)] bg-base-2 p-4 text-sm">
            {segments.map((s) => (
              <div key={s.id} className="flex gap-3">
                <span className="shrink-0 tabular-nums text-fg-muted">{clock(s.t0_ms)}</span>
                <span className="min-w-0 break-words">{s.src_text}</span>
              </div>
            ))}
          </div>
        </div>
      ) : sessions.length === 0 ? (
        <div className="rounded-lg bg-base-2 p-4 text-sm text-fg-muted">{t("history.empty")}</div>
      ) : (
        <div className="flex flex-col gap-1">
          {sessions.map((s) => (
            <button key={s.id} type="button" onClick={() => setSel(s.id)} className="flex flex-wrap items-center gap-2 rounded-md px-3 py-2 text-left text-sm hover:bg-surface">
              <span className="font-semibold">{when(s.started_at)}</span>
              <span className="text-xs text-fg-muted">{duration(s)} · {s.src_lang.toUpperCase()} → {s.tgt_lang.toUpperCase()} · {t("history.segments", { count: s.segments })}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
