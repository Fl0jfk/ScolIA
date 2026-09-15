"use client";

import { useCallback, useEffect, useState } from "react";

type AbsenceRow = {
  conventionId: string;
  studentName: string;
  className: string;
  companyName: string;
  date: string;
  status: string;
  secteur?: string | null;
  secteurLabel?: string | null;
};

type DayBucket = { date: string; absences: AbsenceRow[] };

type SecteurFilter = "" | "college" | "lycee";

function todayIsoLocal(): string {
  const d = new Date();
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

function formatDateFr(iso: string): string {
  return new Date(`${iso}T12:00:00`).toLocaleDateString("fr-FR", {
    weekday: "long",
    day: "numeric",
    month: "long",
  });
}

export default function StageRepasAbsencesPanel({
  onOpenConvention,
}: {
  onOpenConvention?: (conventionId: string) => void;
}) {
  const [days, setDays] = useState<DayBucket[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [from, setFrom] = useState(todayIsoLocal());
  const [to, setTo] = useState(() => {
    const d = new Date(`${todayIsoLocal()}T12:00:00`);
    d.setDate(d.getDate() + 14);
    return d.toISOString().slice(0, 10);
  });
  const [secteur, setSecteur] = useState<SecteurFilter>("");
  const [exporting, setExporting] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const qs = new URLSearchParams();
      if (from) qs.set("from", from);
      if (to) qs.set("to", to);
      if (secteur) qs.set("secteur", secteur);
      const res = await fetch(`/api/stages/repas-absences?${qs}`, { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Chargement impossible");
      setDays(data.days || []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, [from, to, secteur]);

  useEffect(() => {
    void load();
  }, [load]);

  function setTodayOnly() {
    const t = todayIsoLocal();
    setFrom(t);
    setTo(t);
  }

  async function exportPdf() {
    setExporting(true);
    setError(null);
    try {
      const { jsPDF } = await import("jspdf");
      const autoTable = (await import("jspdf-autotable")).default;
      const doc = new jsPDF({ orientation: "portrait", unit: "mm", format: "a4" });
      const secteurLabel =
        secteur === "college" ? "Collège" : secteur === "lycee" ? "Lycée" : "Tous établissements";
      const periodLabel =
        from === to
          ? new Date(`${from}T12:00:00`).toLocaleDateString("fr-FR")
          : `${new Date(`${from}T12:00:00`).toLocaleDateString("fr-FR")} → ${new Date(`${to}T12:00:00`).toLocaleDateString("fr-FR")}`;

      doc.setFontSize(14);
      doc.setFont("helvetica", "bold");
      doc.text("Absences repas — élèves en stage", 14, 18);
      doc.setFontSize(10);
      doc.setFont("helvetica", "normal");
      doc.text(`${secteurLabel} · ${periodLabel}`, 14, 26);
      doc.setFontSize(9);
      doc.setTextColor(100);
      const total = days.reduce((n, d) => n + d.absences.length, 0);
      doc.text(`Généré le ${new Date().toLocaleString("fr-FR")} · ${total} absence(s)`, 14, 32);
      doc.setTextColor(0);

      const body: string[][] = [];
      for (const day of days) {
        for (const a of day.absences) {
          body.push([
            new Date(`${a.date}T12:00:00`).toLocaleDateString("fr-FR"),
            a.studentName,
            a.className,
            a.secteurLabel || "—",
            a.companyName,
          ]);
        }
      }

      autoTable(doc, {
        startY: 38,
        head: [["Date", "Élève", "Classe", "Établissement", "Entreprise"]],
        body: body.length > 0 ? body : [["—", "Aucune absence", "—", "—", "—"]],
        styles: { fontSize: 8, cellPadding: 2 },
        headStyles: { fillColor: [47, 107, 74], textColor: 255 },
        alternateRowStyles: { fillColor: [246, 248, 245] },
      });

      const fileSuffix = from === to ? from : `${from}_${to}`;
      doc.save(`absences-repas-stages_${fileSuffix}.pdf`);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Export PDF impossible");
    } finally {
      setExporting(false);
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-end gap-3">
        <label className="text-xs font-semibold text-stone-600">
          Du
          <input
            type="date"
            className="mt-1 block rounded-lg border px-2 py-1.5 text-sm"
            value={from}
            onChange={(e) => setFrom(e.target.value)}
          />
        </label>
        <label className="text-xs font-semibold text-stone-600">
          Au
          <input
            type="date"
            className="mt-1 block rounded-lg border px-2 py-1.5 text-sm"
            value={to}
            onChange={(e) => setTo(e.target.value)}
          />
        </label>
        <label className="text-xs font-semibold text-stone-600">
          Établissement
          <select
            className="mt-1 block rounded-lg border px-2 py-1.5 text-sm"
            value={secteur}
            onChange={(e) => setSecteur(e.target.value as SecteurFilter)}
          >
            <option value="">Tous</option>
            <option value="college">Collège</option>
            <option value="lycee">Lycée</option>
          </select>
        </label>
        <button
          type="button"
          onClick={setTodayOnly}
          className="rounded-lg border border-[#2F6B4A] px-3 py-2 text-sm font-semibold text-[#2F6B4A]"
        >
          Aujourd&apos;hui
        </button>
        <button
          type="button"
          onClick={() => void load()}
          className="rounded-lg bg-[#2F6B4A] px-3 py-2 text-sm font-semibold text-white"
        >
          Actualiser
        </button>
        <button
          type="button"
          disabled={exporting || loading}
          onClick={() => void exportPdf()}
          className="rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm font-semibold text-stone-800 disabled:opacity-50"
        >
          {exporting ? "Export…" : "Exporter PDF"}
        </button>
      </div>

      {loading && <p className="text-sm text-stone-500">Chargement…</p>}
      {error && <p className="text-sm text-rose-700">{error}</p>}

      {!loading && days.length === 0 && (
        <p className="rounded-lg border border-stone-200 bg-stone-50 px-3 py-2 text-sm text-stone-600">
          Aucune absence stage (repas) sur la période.
        </p>
      )}

      <div className="space-y-3">
        {days.map((day) => (
          <div key={day.date} className="rounded-xl border border-amber-200 bg-amber-50/60 p-4">
            <h3 className="text-sm font-bold capitalize text-amber-950">
              {formatDateFr(day.date)}
              <span className="ml-2 font-normal text-amber-800/80">
                — {day.absences.length} élève{day.absences.length > 1 ? "s" : ""}
              </span>
            </h3>
            <ul className="mt-2 divide-y divide-amber-100">
              {day.absences.map((a) => (
                <li
                  key={`${a.conventionId}-${a.date}`}
                  className="flex flex-wrap items-baseline gap-2 py-1.5 text-sm"
                >
                  {onOpenConvention ? (
                    <button
                      type="button"
                      onClick={() => onOpenConvention(a.conventionId)}
                      className="font-semibold text-[#2F6B4A] underline hover:no-underline"
                    >
                      {a.studentName}
                    </button>
                  ) : (
                    <span className="font-semibold text-stone-900">{a.studentName}</span>
                  )}
                  <span className="text-stone-500">{a.className}</span>
                  {a.secteurLabel ? (
                    <span className="rounded bg-white/80 px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-stone-600">
                      {a.secteurLabel}
                    </span>
                  ) : null}
                  <span className="text-xs text-stone-500">· {a.companyName}</span>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </div>
  );
}
