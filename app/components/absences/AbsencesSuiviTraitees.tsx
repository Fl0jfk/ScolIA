"use client";

import { useMemo, useState } from "react";
import { formatAbsencePeriod } from "@/app/lib/absence-period";
import { describeAbsenceOutcome } from "@/app/lib/absences-outcome";
import type { AbsenceItem } from "@/app/lib/absences-page-model";

type Filter = "toutes" | "validees" | "refusees" | "en-traitement";

function normalize(s: string) {
  return s.toLowerCase().normalize("NFD").replace(/[̀-ͯ]/g, "");
}

type Props = {
  items: AbsenceItem[];
  loading: boolean;
};

/**
 * Historique des demandes d'autorisation déjà décidées : qui a validé, quand,
 * et comment le dossier s'est terminé. Lecture seule : la liste ne contient que
 * les absences que le serveur autorise déjà ce lecteur à voir.
 */
export default function AbsencesSuiviTraitees({ items, loading }: Props) {
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<Filter>("toutes");

  const rows = useMemo(() => {
    const q = normalize(query.trim());
    return items
      .map((item) => ({ item, outcome: describeAbsenceOutcome(item) }))
      .filter(({ item, outcome }) => {
        if (filter === "validees" && outcome.state !== "validee") return false;
        if (filter === "refusees" && outcome.state !== "refusee") return false;
        if (filter === "en-traitement" && (outcome.state !== "validee" || outcome.closed)) return false;
        if (!q) return true;
        const hay = normalize(
          [item.createdBy.name, item.data.reason, item.data.etablissement || "", outcome.decisionLine].join(" "),
        );
        return hay.includes(q);
      })
      .sort((a, b) => String(b.item.data.startDate).localeCompare(String(a.item.data.startDate)));
  }, [items, query, filter]);

  const filters: Array<{ id: Filter; label: string }> = [
    { id: "toutes", label: "Toutes" },
    { id: "validees", label: "Validées" },
    { id: "en-traitement", label: "En traitement RH" },
    { id: "refusees", label: "Refusées" },
  ];

  return (
    <div className="space-y-4">
      <div className="bg-white border border-slate-200 rounded-3xl p-4 space-y-3">
        <div>
          <h3 className="font-black text-slate-900">Suivi des absences traitées</h3>
          <p className="text-xs text-slate-500">
            Demandes déjà décidées : qui les a validées ou refusées, quand, et comment le dossier s&apos;est terminé.
            Une absence en rattrapage interne est clôturée dès la validation : elle n&apos;apparaît plus dans
            aucune liste « à traiter », mais elle est ici.
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Rechercher un nom, un motif…"
            className="min-w-[220px] flex-1 rounded-xl border border-slate-200 px-3 py-2 text-sm"
            aria-label="Rechercher dans les absences traitées"
          />
          <div className="flex flex-wrap gap-1" role="group" aria-label="Filtrer par décision">
            {filters.map((f) => (
              <button
                key={f.id}
                type="button"
                onClick={() => setFilter(f.id)}
                aria-pressed={filter === f.id}
                className={[
                  "rounded-full border px-3 py-1.5 text-xs font-bold transition",
                  filter === f.id
                    ? "border-slate-900 bg-slate-900 text-white"
                    : "border-slate-200 bg-white text-slate-700 hover:bg-slate-50",
                ].join(" ")}
              >
                {f.label}
              </button>
            ))}
          </div>
        </div>
      </div>

      {loading ? (
        <div className="bg-white border border-slate-200 rounded-3xl p-8 text-slate-500">Chargement…</div>
      ) : rows.length === 0 ? (
        <div className="bg-white border border-dashed border-slate-300 rounded-3xl p-8 text-slate-500">
          Aucune absence traitée ne correspond.
        </div>
      ) : (
        <ul className="space-y-2">
          {rows.map(({ item, outcome }) => {
            const tone =
              outcome.state === "refusee"
                ? "bg-rose-600 text-white"
                : outcome.closed
                  ? "bg-emerald-600 text-white"
                  : "bg-emerald-50 text-emerald-800 border border-emerald-200";
            return (
              <li key={item.id} className="bg-white border border-slate-200 rounded-2xl p-4">
                <div className="flex flex-wrap items-start justify-between gap-2">
                  <div className="min-w-0">
                    <p className="font-black text-slate-900">
                      {item.createdBy.name}
                      {item.data.etablissement ? (
                        <span className="ml-2 text-xs font-semibold text-slate-500">{item.data.etablissement}</span>
                      ) : null}
                    </p>
                    <p className="text-xs text-slate-500">
                      {formatAbsencePeriod(item.data)}
                      {item.data.reason ? ` • ${item.data.reason}` : ""}
                    </p>
                  </div>
                  <span className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[11px] font-black ${tone}`}>
                    <span aria-hidden>{outcome.state === "refusee" ? "✕" : "✓"}</span>
                    {outcome.shortLabel}
                  </span>
                </div>
                <p className="mt-2 text-sm font-semibold text-slate-800">{outcome.decisionLine}</p>
                {outcome.closureLine ? <p className="text-sm text-slate-600">{outcome.closureLine}</p> : null}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
