import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router";
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
  const [params] = useSearchParams();
  const [q, setQ] = useState(params.get("q") ?? "");
  // 사이드바 검색은 ?q= 로 들어온다. 파라미터가 바뀌면 검색어도 따라간다.
  useEffect(() => { const v = params.get("q"); if (v !== null) setQ(v); }, [params]);
  const [hits, setHits] = useState<SegmentRow[] | null>(null);
  const [saved, setSaved] = useState<string | null>(null);
  const toastTimer = useRef<number | undefined>(undefined);

  // history 커맨드는 DB 상태가 없으면 "state not managed"로 거절한다. 원문 대신 사람 말로.
  const fail = (e: unknown) => {
    const msg = e instanceof Error ? e.message : String(e);
    setError(msg.includes("state not managed") ? t("errors.historyUnavailable") : msg);
  };

  const load = () => api.historySessions(100).then(setSessions).catch(fail);
  useEffect(() => { load(); }, []);
  useEffect(() => () => window.clearTimeout(toastTimer.current), []);

  // 늦게 온 이전 세션의 조각이 지금 화면을 덮지 않게 한다.
  useEffect(() => {
    if (sel === null) { setSegments([]); return; }
    let alive = true;
    api.historySegments(sel).then((r) => { if (alive) setSegments(r); }).catch(fail);
    return () => { alive = false; };
  }, [sel]);

  // 타이핑마다 쿼리를 던지면 DB가 앓는다.
  useEffect(() => {
    const term = q.trim();
    if (!term) { setHits(null); return; }
    const id = window.setTimeout(() => { api.historySearch(term).then(setHits).catch(fail); }, 300);
    return () => window.clearTimeout(id);
  }, [q]);

  const toast = (path: string) => {
    setSaved(path);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setSaved(null), 2000);
  };
  const exportAs = (id: number, format: "txt" | "srt") => api.historyExport(id, format).then(toast).catch(fail);
  const remove = (id: number) => api.historyDelete(id).then(() => { setSel(null); load(); }).catch(fail);

  const when = (epoch: number) => new Date(epoch * 1000).toLocaleString();
  const duration = (s: SessionSummary) => (s.ended_at ? clock((s.ended_at - s.started_at) * 1000) : "—");
  // 검색 결과의 세션이 최근 100개 밖일 수 있다. 요약이 없어도 상세는 연다.
  const current = sessions.find((s) => s.id === sel);
  const sessionLabel = (id: number) => { const s = sessions.find((x) => x.id === id); return s ? when(s.started_at) : `#${id}`; };
  const head = current
    ? `${when(current.started_at)} · ${duration(current)} · ${current.src_lang.toUpperCase()} → ${current.tgt_lang.toUpperCase()} · ${t("history.segments", { count: current.segments })}`
    : `#${sel} · ${clock(segments[segments.length - 1]?.t1_ms ?? 0)} · ${t("history.segments", { count: segments.length })}`;

  return (
    <div className="flex max-w-3xl flex-col gap-4">
      <div className="flex items-center justify-between gap-4">
        <h2 className="text-2xl font-bold">{t("nav.history")}</h2>
        <input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder={t("history.search")}
          aria-label={t("history.search")}
          className="input input-sm w-56 rounded-full"
        />
      </div>

      {saved && <div className="rounded-md bg-neutral px-3 py-2 text-xs text-fg-muted">{t("history.saved", { path: saved })}</div>}

      {hits ? (
        <div className="flex flex-col gap-1">
          {hits.map((r) => (
            <button key={r.id} type="button" onClick={() => { setQ(""); setSel(r.session_id); }} className="flex gap-3 rounded-md px-3 py-2 text-left text-sm hover:bg-base-300">
              <span className="shrink-0 tabular-nums text-fg-muted">{sessionLabel(r.session_id)} · {clock(r.t0_ms)}</span>
              <span className="min-w-0 break-words">
                <span>{r.src_text}</span>
                {r.tgt_text && <span className="block font-bold">{r.tgt_text}</span>}
              </span>
            </button>
          ))}
        </div>
      ) : sel !== null ? (
        <div className="flex flex-col gap-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <button type="button" className="btn btn-ghost btn-sm" onClick={() => setSel(null)}>{`← ${t("nav.history")}`}</button>
            <div className="flex gap-2">
              <button type="button" className="btn btn-neutral btn-sm" onClick={() => exportAs(sel, "txt")}>{t("history.exportTxt")}</button>
              <button type="button" className="btn btn-neutral btn-sm" onClick={() => exportAs(sel, "srt")}>{t("history.exportSrt")}</button>
              <button type="button" className="btn btn-outline btn-sm" onClick={() => remove(sel)}>{t("history.delete")}</button>
            </div>
          </div>
          <div className="text-xs text-fg-muted">{head}</div>
          <div className="flex flex-col gap-2 rounded-box bg-base-200 p-4 text-sm">
            {segments.map((s) => (
              <div key={s.id} className="flex gap-3">
                <span className="shrink-0 tabular-nums text-fg-muted">{clock(s.t0_ms)}</span>
                <div className="min-w-0 break-words">
                  <div>{s.src_text}</div>
                  {s.tgt_text && <div className="font-bold">{s.tgt_text}</div>}
                </div>
              </div>
            ))}
          </div>
        </div>
      ) : sessions.length === 0 ? (
        <div className="rounded-lg bg-base-200 p-4 text-sm text-fg-muted">{t("history.empty")}</div>
      ) : (
        <div className="flex flex-col gap-1">
          {sessions.map((s) => (
            <button key={s.id} type="button" onClick={() => setSel(s.id)} className="flex flex-wrap items-center gap-2 rounded-md px-3 py-2 text-left text-sm hover:bg-base-300">
              <span className="font-semibold">{when(s.started_at)}</span>
              <span className="text-xs text-fg-muted">{duration(s)} · {s.src_lang.toUpperCase()} → {s.tgt_lang.toUpperCase()} · {t("history.segments", { count: s.segments })}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
