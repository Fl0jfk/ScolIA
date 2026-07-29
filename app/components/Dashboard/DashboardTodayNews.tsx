"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { dash } from "@/app/lib/dashboard-brand";
import type { DashboardTodayNewsItem } from "@/app/lib/dashboard-signals";

type Props = {
  items: DashboardTodayNewsItem[];
  hasCurrentWeek: boolean;
  loading?: boolean;
  onWeekSheetUpdated?: () => void;
};

/** Même forme / taille que `DashboardWeather`. */
export default function DashboardTodayNews({
  items,
  hasCurrentWeek,
  loading,
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
  const empty = !loading && (!hasCurrentWeek || items.length === 0);
  const meta = [current?.time, current?.location].filter(Boolean).join(" · ");

  return (
    <div
      className={`flex items-center gap-3 rounded-xl border bg-white/90 px-3 py-2 shadow-sm backdrop-blur-sm ${dash.borderSoft}`}
      aria-label="Actualité du jour"
      title={current?.title || "Actualité du jour"}
    >
      <span className="text-2xl leading-none" aria-hidden>
        📰
      </span>
      <div className="min-w-0 flex-1">
        <p className={`text-[10px] font-bold uppercase tracking-widest ${dash.label}`}>Aujourd&apos;hui</p>

        {loading ? (
          <p className="text-xs font-semibold text-stone-600">Chargement…</p>
        ) : empty ? (
          <>
            <p className={`text-lg font-black leading-tight ${dash.ink}`}>Pas d&apos;actualité</p>
            <p className="text-[10px] font-medium text-stone-400">aujourd&apos;hui</p>
          </>
        ) : (
          <div className="relative min-h-[2.5rem]">
            <AnimatePresence mode="wait">
              <motion.div
                key={current?.id ?? index}
                initial={{ opacity: 0, y: 4 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -4 }}
                transition={{ duration: 0.3, ease: [0.22, 1, 0.36, 1] }}
              >
                <p className="flex flex-wrap items-baseline gap-x-2 gap-y-0">
                  <span className={`truncate text-lg font-black ${dash.ink}`}>{current?.title}</span>
                </p>
                <p className="text-[10px] font-medium text-stone-400">
                  {meta || (items.length > 1 ? `${index + 1} / ${items.length}` : "\u00a0")}
                </p>
              </motion.div>
            </AnimatePresence>
          </div>
        )}

        {error ? <p className="text-[10px] font-medium text-rose-600">{error}</p> : null}
      </div>

      <div className="flex shrink-0 flex-col items-end gap-1">
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
              className="text-[10px] font-bold text-[var(--dash-primary)] hover:underline disabled:opacity-50"
            >
              {importing ? "…" : "PDF"}
            </button>
          </>
        ) : null}
      </div>
    </div>
  );
}
