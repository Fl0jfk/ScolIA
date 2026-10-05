"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { dash } from "@/app/lib/dashboard-brand";
import { DASH_CHIP_SHELL, DASH_NEWS_WIDTH } from "@/app/lib/dashboard-chip";
import type { DashboardTodayNewsItem } from "@/app/lib/dashboard-signals";

type Props = {
  items: DashboardTodayNewsItem[];
  hasCurrentWeek: boolean;
  loading?: boolean;
  onWeekSheetUpdated?: () => void;
  /** Largeur fixe élargie (~+50 % vs 2× météo ; défaut true hors mobile full-bleed). */
  wide?: boolean;
  /** Variante dense pour la sidebar intranet. */
  variant?: "chip" | "sidebar";
};

function isPdfFile(file: File): boolean {
  const type = (file.type || "").toLowerCase();
  const name = (file.name || "").toLowerCase();
  return type === "application/pdf" || type === "application/x-pdf" || name.endsWith(".pdf");
}

/** Même hauteur / forme que `DashboardWeather` ; largeur élargie (~+50 %). */
export default function DashboardTodayNews({
  items,
  hasCurrentWeek,
  loading,
  onWeekSheetUpdated,
  wide = true,
  variant = "chip",
}: Props) {
  const isOrgAdmin = useIsOrgAdmin();
  const fileRef = useRef<HTMLInputElement>(null);
  const [index, setIndex] = useState(0);
  const [importing, setImporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  useEffect(() => {
    setIndex(0);
  }, [items.map((i) => i.id).join("|")]);

  useEffect(() => {
    if (items.length <= 1) return;
    const t = window.setInterval(() => {
      setIndex((i) => (i + 1) % items.length);
    }, 5200);
    return () => window.clearInterval(t);
  }, [items.length]);

  useEffect(() => {
    if (!success) return;
    const t = window.setTimeout(() => setSuccess(null), 10_000);
    return () => window.clearTimeout(t);
  }, [success]);

  const handleFile = useCallback(
    async (file: File) => {
      if (!file || !isPdfFile(file)) {
        setError("Choisissez un fichier PDF.");
        setSuccess(null);
        return;
      }
      setImporting(true);
      setError(null);
      setSuccess(null);
      try {
        const fd = new FormData();
        fd.append("file", file, file.name || "feuille-semaine.pdf");
        const imp = await fetch("/api/dashboard/week-sheet/import", {
          method: "POST",
          body: fd,
        });
        const impJson = (await imp.json().catch(() => ({}))) as {
          error?: string;
          eventCount?: number;
          todayEventCount?: number;
          hasCurrentWeek?: boolean;
          weekLabel?: string | null;
        };
        if (!imp.ok) {
          throw new Error(impJson.error || "Analyse impossible.");
        }
        const weekCount = Number(impJson.eventCount ?? 0);
        const todayCount = Number(impJson.todayEventCount ?? 0);
        const label = impJson.weekLabel?.trim();
        if (todayCount > 0) {
          setSuccess(
            `${todayCount} actu${todayCount > 1 ? "s" : ""} aujourd’hui${label ? ` · ${label}` : ""}`,
          );
        } else if (weekCount > 0 && impJson.hasCurrentWeek) {
          setSuccess(
            `Semaine OK (${weekCount} créneaux) — rien pour aujourd’hui${label ? ` · ${label}` : ""}`,
          );
        } else if (weekCount > 0) {
          setSuccess(
            `${weekCount} créneau${weekCount > 1 ? "x" : ""} trouvés, mais pas la semaine en cours${label ? ` · ${label}` : ""}`,
          );
        } else {
          setSuccess("PDF importé — aucun créneau reconnu.");
        }
        onWeekSheetUpdated?.();
      } catch (e) {
        setError(e instanceof Error ? e.message : "Erreur inconnue.");
      } finally {
        setImporting(false);
        if (fileRef.current) fileRef.current.value = "";
      }
    },
    [onWeekSheetUpdated],
  );

  const current = items[index];
  const empty = !loading && (!hasCurrentWeek || items.length === 0);
  const meta = [current?.time, current?.location].filter(Boolean).join(" · ");
  const statusLine = importing
    ? "Analyse du PDF…"
    : error
      ? error
      : success
        ? success
        : null;

  /** Sidebar : toujours réserver 2 lignes (leading-snug = 1.375 → 2.75em) pour éviter le layout shift. */
  const sidebarTitleClass =
    variant === "sidebar"
      ? "line-clamp-2 min-h-[2.75em] text-[15px]"
      : "truncate text-lg";

  const body = loading || importing ? (
    <>
      <p className={`font-black leading-snug ${sidebarTitleClass} ${dash.ink}`}>
        {importing ? "Import…" : "…"}
      </p>
      {variant !== "sidebar" ? (
        <p className="truncate text-[10px] font-medium leading-tight text-stone-400">
          {importing ? "OCR + analyse" : "chargement"}
        </p>
      ) : (
        <p className="truncate text-[10px] font-medium leading-tight text-stone-400">
          {"\u00a0"}
        </p>
      )}
    </>
  ) : empty ? (
    <>
      <p className={`font-black leading-snug ${sidebarTitleClass} ${dash.ink}`}>
        Pas d&apos;actualité
      </p>
      <p
        className={`truncate text-[10px] font-medium leading-tight ${
          error ? "text-rose-600" : success ? "text-emerald-700" : "text-stone-400"
        }`}
        title={statusLine || "aujourd'hui"}
      >
        {statusLine || (variant === "sidebar" ? "\u00a0" : "aujourd'hui")}
      </p>
    </>
  ) : (
    <AnimatePresence mode="wait">
      <motion.div
        key={current?.id ?? index}
        initial={{ opacity: 0, y: 3 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, y: -3 }}
        transition={{ duration: 0.28 }}
      >
        <p className={`font-black leading-snug ${sidebarTitleClass} ${dash.ink}`}>
          {current?.title}
        </p>
        <p
          className={`truncate text-[10px] font-medium leading-tight ${
            error ? "text-rose-600" : success ? "text-emerald-700" : "text-stone-400"
          }`}
          title={statusLine || meta || undefined}
        >
          {statusLine || meta || "\u00a0"}
        </p>
      </motion.div>
    </AnimatePresence>
  );

  const fileInput = isOrgAdmin ? (
    <input
      ref={fileRef}
      type="file"
      accept="application/pdf,.pdf"
      className="hidden"
      disabled={importing}
      onChange={(e) => {
        const f = e.target.files?.[0];
        if (f) void handleFile(f);
      }}
    />
  ) : null;

  const chipControls =
    items.length > 1 || isOrgAdmin ? (
      <div className="flex shrink-0 flex-col items-end justify-center gap-1 self-stretch">
        {items.length > 1 ? (
          <div className="flex items-center gap-1">
            {items.map((item, i) => (
              <button
                key={item.id}
                type="button"
                aria-label={`Actualité ${i + 1}`}
                onClick={() => setIndex(i)}
                className={`h-1 rounded-full transition-all ${
                  i === index ? "w-3 bg-[var(--dash-primary)]" : "w-1.5 bg-stone-300"
                }`}
              />
            ))}
          </div>
        ) : null}
        {isOrgAdmin ? (
          <button
            type="button"
            disabled={importing}
            onClick={() => fileRef.current?.click()}
            className="text-[10px] font-bold leading-none text-[var(--dash-primary)] hover:underline disabled:opacity-50"
            title="Importer la feuille de semaine (PDF)"
          >
            {importing ? "…" : "PDF"}
          </button>
        ) : null}
      </div>
    ) : null;

  if (variant === "sidebar") {
    return (
      <div
        className="flex w-full items-start gap-2 overflow-hidden px-0.5 py-0"
        aria-label="Actualité du jour"
        title={error || success || current?.title || "Actualité du jour"}
      >
        {fileInput}
        {isOrgAdmin ? (
          <button
            type="button"
            disabled={importing}
            onClick={() => fileRef.current?.click()}
            className="mt-0.5 shrink-0 text-xl leading-none transition hover:scale-105 disabled:opacity-50"
            title="Importer la feuille de semaine (PDF)"
            aria-label="Importer un PDF d'actualités"
          >
            📰
          </button>
        ) : (
          <span className="mt-0.5 shrink-0 text-xl leading-none" aria-hidden>
            📰
          </span>
        )}
        <div className="min-w-0 flex-1 overflow-hidden text-[15px]">
          {/* Titre 2 lignes (2.75em) + méta (~0.83em à 10px) : hauteur figée même à 1 ligne */}
          <div className="min-h-[calc(2.75em+0.833em)]">{body}</div>
        </div>
      </div>
    );
  }

  return (
    <div
      className={`${DASH_CHIP_SHELL} ${wide ? DASH_NEWS_WIDTH : "w-full"}`}
      aria-label="Actualité du jour"
      title={error || success || current?.title || "Actualité du jour"}
    >
      {fileInput}
      <span className="text-2xl leading-none" aria-hidden>
        📰
      </span>
      <div className="min-w-0 flex-1 overflow-hidden">
        <p className={`text-[10px] font-bold uppercase tracking-widest ${dash.label}`}>
          Aujourd&apos;hui
        </p>
        {body}
      </div>
      {chipControls}
    </div>
  );
}
