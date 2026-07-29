"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import type { DashboardTodayNewsItem } from "@/app/lib/dashboard-signals";

type Props = {
  items: DashboardTodayNewsItem[];
  hasCurrentWeek: boolean;
  loading?: boolean;
  /** Version compacte pour la barre header (tablette / desktop). */
  compact?: boolean;
  onWeekSheetUpdated?: () => void;
};

export default function DashboardTodayNews({
  items,
  hasCurrentWeek,
  loading,
  compact = false,
  onWeekSheetUpdated,
}: Props) {
  const isOrgAdmin = useIsOrgAdmin();
  const fileRef = useRef<HTMLInputElement>(null);
  const [index, setIndex] = useState(0);
  const [importing, setImporting] = useState(false);
  const [error, setError] = useState<string | null>(null);

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

  const handleFile = useCallback(
    async (file: File) => {
      if (!file || file.type !== "application/pdf") {
        setError("Choisissez un fichier PDF.");
        return;
      }
      setImporting(true);
      setError(null);
      try {
        const prep = await fetch("/api/dashboard/week-sheet/upload-url", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ fileName: file.name }),
        });
        const prepJson = await prep.json();
        if (!prep.ok) throw new Error(prepJson.error || "Préparation upload impossible.");

        const put = await fetch(prepJson.uploadUrl, {
          method: "PUT",
          headers: { "Content-Type": "application/pdf" },
          body: file,
        });
        if (!put.ok) throw new Error("Envoi du PDF échoué.");

        const imp = await fetch("/api/dashboard/week-sheet/import", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ key: prepJson.key }),
        });
        const impJson = await imp.json();
        if (!imp.ok) throw new Error(impJson.error || "Analyse impossible.");
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

  const body = (
    <>
      {loading ? (
        <p className={`text-[var(--dash-mid)] ${compact ? "text-xs" : "mt-2 text-sm"}`}>Chargement…</p>
      ) : !hasCurrentWeek || items.length === 0 ? (
        <p className={`font-medium text-stone-500 ${compact ? "truncate text-sm" : "mt-2 text-[15px]"}`}>
          Pas d&apos;actualité aujourd&apos;hui
        </p>
      ) : (
        <div className={`relative ${compact ? "min-h-[1.5rem]" : "mt-1.5 min-h-[3rem]"}`}>
          <AnimatePresence mode="wait">
            <motion.div
              key={current?.id ?? index}
              initial={{ opacity: 0, y: compact ? 4 : 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: compact ? -4 : -8 }}
              transition={{ duration: 0.35, ease: [0.22, 1, 0.36, 1] }}
            >
              <p
                className={`truncate font-semibold tracking-tight text-[var(--dash-ink)] ${
                  compact ? "text-sm sm:text-[15px]" : "text-lg sm:text-xl"
                }`}
              >
                {current?.title}
              </p>
              {!compact ? (
                <p className="mt-0.5 truncate text-sm text-[var(--dash-mid)]">
                  {[current?.time, current?.location].filter(Boolean).join(" · ") || "\u00a0"}
                </p>
              ) : current?.time || current?.location ? (
                <p className="truncate text-[11px] text-[var(--dash-mid)]">
                  {[current?.time, current?.location].filter(Boolean).join(" · ")}
                </p>
              ) : null}
            </motion.div>
          </AnimatePresence>
        </div>
      )}
    </>
  );

  return (
    <section
      className={
        compact
          ? "relative min-w-0 overflow-hidden rounded-2xl border border-white/60 bg-white/55 px-3.5 py-2 shadow-[0_8px_30px_-22px_rgba(15,23,42,0.3)] backdrop-blur-xl"
          : "relative overflow-hidden rounded-[1.35rem] border border-white/60 bg-white/55 px-5 py-4 shadow-[0_8px_40px_-24px_rgba(15,23,42,0.35)] backdrop-blur-2xl sm:px-6"
      }
      aria-label="Actualité du jour"
    >
      <div className={`relative flex items-start justify-between gap-2 ${compact ? "items-center" : ""}`}>
        <div className="min-w-0 flex-1">
          <p className="text-[9px] font-semibold uppercase tracking-[0.2em] text-[var(--dash-mid)]">
            Aujourd&apos;hui
          </p>
          {body}
        </div>

        <div className="flex shrink-0 items-center gap-2">
          {items.length > 1 ? (
            <div className={`flex items-center gap-1 ${compact ? "" : "absolute bottom-0 left-0"}`}>
              {!compact ? null : (
                <>
                  {items.map((item, i) => (
                    <button
                      key={item.id}
                      type="button"
                      aria-label={`Actualité ${i + 1}`}
                      onClick={() => setIndex(i)}
                      className={`h-1 cursor-pointer rounded-full transition-all ${
                        i === index ? "w-4 bg-[var(--dash-primary)]" : "w-1.5 bg-[color:var(--dash-border)]"
                      }`}
                    />
                  ))}
                </>
              )}
            </div>
          ) : null}
          {isOrgAdmin ? (
            <>
              <input
                ref={fileRef}
                type="file"
                accept="application/pdf"
                className="hidden"
                onChange={(e) => {
                  const f = e.target.files?.[0];
                  if (f) void handleFile(f);
                }}
              />
              <button
                type="button"
                disabled={importing}
                onClick={() => fileRef.current?.click()}
                className="cursor-pointer rounded-full border border-white/70 bg-white/70 px-2.5 py-1 text-[10px] font-semibold text-[var(--dash-primary)] shadow-sm backdrop-blur transition hover:bg-white disabled:opacity-50"
              >
                {importing ? "…" : "PDF"}
              </button>
            </>
          ) : null}
        </div>
      </div>

      {!compact && items.length > 1 ? (
        <div className="relative mt-3 flex items-center gap-1.5">
          {items.map((item, i) => (
            <button
              key={item.id}
              type="button"
              aria-label={`Actualité ${i + 1}`}
              onClick={() => setIndex(i)}
              className={`h-1 cursor-pointer rounded-full transition-all duration-500 ${
                i === index
                  ? "w-6 bg-[var(--dash-primary)]"
                  : "w-1.5 bg-[color:var(--dash-border)] hover:bg-[var(--dash-mid)]"
              }`}
            />
          ))}
        </div>
      ) : null}

      {error ? <p className="relative mt-1 text-[10px] font-medium text-rose-600">{error}</p> : null}
    </section>
  );
}
