"use client";

import { useEffect, useMemo, useState } from "react";
import { createPortal } from "react-dom";
import { AnimatePresence, motion } from "framer-motion";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import { parseTimeToMinutes } from "@/app/lib/dashboard-week-sheet-time";
import {
  WEEK_DAYS,
  type WeekDayKey,
  type WeekSheetData,
  type WeekSheetEvent,
} from "@/app/lib/dashboard-week-sheet-types";

type Props = {
  open: boolean;
  onClose: () => void;
};

function addDays(key: string, days: number): string {
  const [y, m, d] = key.split("-").map(Number);
  const next = new Date(Date.UTC(y, m - 1, d + days, 12, 0, 0));
  return next.toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
}

function todayWeekDay(): WeekDayKey | null {
  const wd = new Intl.DateTimeFormat("en-US", { timeZone: "Europe/Paris", weekday: "short" }).format(
    new Date(),
  );
  const map: Record<string, WeekDayKey> = { Mon: "mon", Tue: "tue", Wed: "wed", Thu: "thu", Fri: "fri" };
  return map[wd] ?? null;
}

function shortDate(key: string): string {
  const [y, m, d] = key.split("-").map(Number);
  return new Date(Date.UTC(y, m - 1, d, 12)).toLocaleDateString("fr-FR", {
    day: "numeric",
    month: "short",
    timeZone: "Europe/Paris",
  });
}

function eventTime(ev: WeekSheetEvent): string | null {
  const start = ev.startTime?.trim();
  if (!start) return null;
  const end = ev.endTime?.trim();
  if (!end || parseTimeToMinutes(end) === null) return start;
  return `${start} – ${end}`;
}

function sortEvents(events: WeekSheetEvent[]): WeekSheetEvent[] {
  return [...events].sort(
    (a, b) => (parseTimeToMinutes(a.startTime) ?? 9999) - (parseTimeToMinutes(b.startTime) ?? 9999),
  );
}

function useIsMobile(): boolean {
  const [mobile, setMobile] = useState(false);
  useEffect(() => {
    const media = window.matchMedia("(max-width: 639px)");
    const update = () => setMobile(media.matches);
    update();
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);
  return mobile;
}

/** Aperçu léger de la feuille de semaine : la semaine sur tablette/bureau, le jour sur mobile. */
export default function WeekSheetQuickView({ open, onClose }: Props) {
  const isMobile = useIsMobile();
  const [mounted, setMounted] = useState(false);
  const [data, setData] = useState<WeekSheetData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => setMounted(true), []);

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setLoading(true);
    setError(null);
    fetch("/api/dashboard/week-sheet", { cache: "no-store" })
      .then(async (res) => {
        const json = (await res.json().catch(() => ({}))) as { data?: WeekSheetData | null; error?: string };
        if (!res.ok) throw new Error(json.error || "Chargement impossible.");
        if (!cancelled) setData(json.data ?? null);
      })
      .catch((e) => {
        if (!cancelled) setError(e instanceof Error ? e.message : "Chargement impossible.");
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  const view = useMemo(() => {
    const todayKey = calendarDateKeyParis();
    const weekStart = data?.weekStart && /^\d{4}-\d{2}-\d{2}$/.test(data.weekStart) ? data.weekStart : null;
    const isCurrentWeek = weekStart ? todayKey >= weekStart && todayKey <= addDays(weekStart, 4) : false;
    const today = isCurrentWeek ? todayWeekDay() : null;
    const days = WEEK_DAYS.map((d, i) => ({
      ...d,
      date: weekStart ? addDays(weekStart, i) : null,
      isToday: today === d.key,
      events: sortEvents((data?.events ?? []).filter((ev) => ev.day === d.key)),
    }));
    return { days, isCurrentWeek, today, weekStart };
  }, [data]);

  const mobileDay = isMobile ? view.days.find((d) => d.isToday) ?? null : null;
  const visibleDays = isMobile ? (mobileDay ? [mobileDay] : []) : view.days;
  const totalEvents = data?.events?.length ?? 0;

  const title = isMobile ? "Aujourd’hui" : "Feuille de semaine";
  const subtitle = data?.weekLabel?.trim()
    || (view.weekStart ? `Semaine du ${shortDate(view.weekStart)}` : null);

  if (!mounted) return null;

  return createPortal(
    <AnimatePresence>
      {open ? (
        <motion.div
          key="week-sheet-quick-view"
          className="fixed inset-0 z-[80] flex items-start justify-center overflow-y-auto bg-black/30 p-3 pt-16 sm:items-center sm:p-6 print:hidden"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.15 }}
          onMouseDown={(e) => {
            if (e.target === e.currentTarget) onClose();
          }}
        >
          <motion.div
            role="dialog"
            aria-modal="true"
            aria-label={title}
            className={`w-full rounded-3xl bg-[#f7f7f4] p-4 shadow-2xl ring-1 ring-black/5 sm:p-5 ${
              isMobile ? "max-w-md" : "max-w-5xl"
            }`}
            initial={{ opacity: 0, y: 8, scale: 0.98 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 8, scale: 0.98 }}
            transition={{ duration: 0.18 }}
          >
            <div className="mb-3 flex items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="text-[10px] font-bold uppercase tracking-widest text-[var(--dash-mid)]">
                  📰 {title}
                </p>
                {subtitle ? (
                  <p className="truncate text-base font-black text-[var(--dash-ink)]">{subtitle}</p>
                ) : null}
                {data && !loading && !view.isCurrentWeek && totalEvents > 0 ? (
                  <p className="text-xs font-medium text-amber-700">
                    Ce n’est pas la semaine en cours : dernière feuille importée.
                  </p>
                ) : null}
              </div>
              <button
                type="button"
                onClick={onClose}
                className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-white text-[var(--dash-ink)] shadow-sm hover:bg-stone-100"
                aria-label="Fermer"
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} className="h-4 w-4" aria-hidden>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M6 18 18 6M6 6l12 12" />
                </svg>
              </button>
            </div>

            {loading ? (
              <p className="py-8 text-center text-sm text-stone-500">Chargement…</p>
            ) : error ? (
              <p className="py-8 text-center text-sm text-rose-600">{error}</p>
            ) : totalEvents === 0 ? (
              <p className="py-8 text-center text-sm text-stone-500">Aucune feuille de semaine importée.</p>
            ) : isMobile && !mobileDay ? (
              <p className="py-8 text-center text-sm text-stone-500">
                Rien de prévu aujourd’hui sur la feuille de semaine.
              </p>
            ) : (
              <div
                className="grid gap-2"
                style={{ gridTemplateColumns: `repeat(${visibleDays.length}, minmax(0, 1fr))` }}
              >
                {visibleDays.map((d) => (
                  <section
                    key={d.key}
                    className={`flex min-w-0 flex-col rounded-2xl p-2 ${
                      d.isToday ? "bg-white ring-2 ring-[var(--dash-primary)]" : "bg-white/70"
                    }`}
                  >
                    <header className="mb-1.5 flex items-baseline justify-between gap-1 px-1">
                      <span
                        className={`text-xs font-black uppercase tracking-wide ${
                          d.isToday ? "text-[var(--dash-primary)]" : "text-[var(--dash-ink)]"
                        }`}
                      >
                        {isMobile ? d.label : d.short}
                      </span>
                      {d.date ? (
                        <span className="text-[10px] font-medium text-stone-400">{shortDate(d.date)}</span>
                      ) : null}
                    </header>
                    {d.events.length === 0 ? (
                      <p className="px-1 py-2 text-[11px] text-stone-400">—</p>
                    ) : (
                      <ul className="space-y-1.5 overflow-y-auto sm:max-h-[60vh]">
                        {d.events.map((ev) => {
                          const time = eventTime(ev);
                          return (
                            <li key={ev.id} className="rounded-xl bg-[#f1f2ee] px-2 py-1.5">
                              {time ? (
                                <p className="text-[10px] font-bold tabular-nums text-[var(--dash-primary)]">
                                  {time}
                                </p>
                              ) : null}
                              <p className="break-words text-[12px] font-semibold leading-snug text-[var(--dash-ink)]">
                                {ev.title}
                              </p>
                              {ev.location ? (
                                <p className="break-words text-[10px] text-stone-500">📍 {ev.location}</p>
                              ) : null}
                            </li>
                          );
                        })}
                      </ul>
                    )}
                  </section>
                ))}
              </div>
            )}
          </motion.div>
        </motion.div>
      ) : null}
    </AnimatePresence>,
    document.body,
  );
}
