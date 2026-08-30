"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";

type AbsenceRow = {
  id: string;
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  dateDebut: string;
  type: string;
  statut: string;
  justifie: boolean;
  motif: string | null;
  noteCpe: string | null;
  relanceAt: string | null;
  source?: string | null;
  interne?: boolean;
  internatStudentId?: string | null;
};

type FiltreJustif = "tous" | "justif_famille" | "sans_motif";

function parseFiltreJustif(raw: string | null): FiltreJustif {
  if (raw === "justif_famille" || raw === "sans_motif") return raw;
  return "tous";
}

export default function VsAbsencesClient({ embedded = false }: { embedded?: boolean }) {
  const searchParams = useSearchParams();
  const router = useRouter();
  const pathname = usePathname();
  const [absences, setAbsences] = useState<AbsenceRow[]>([]);
  const [aTraiter, setATraiter] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const filtreJustif = parseFiltreJustif(searchParams.get("filtre"));

  const setFiltreJustif = (value: FiltreJustif) => {
    const params = new URLSearchParams(searchParams.toString());
    if (value === "tous") params.delete("filtre");
    else params.set("filtre", value);
    const q = params.toString();
    router.replace(q ? `${pathname}?${q}` : pathname, { scroll: false });
  };

  const load = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/vie-scolaire/absences?statut=a_traiter", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setAbsences(data.absences || []);
      setATraiter(data.counts?.aTraiter ?? 0);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const withParentJustif = useMemo(
    () => absences.filter((a) => Boolean(a.motif?.trim()) && !a.justifie),
    [absences],
  );

  const visible = useMemo(() => {
    let rows = [...absences];
    if (filtreJustif === "justif_famille") {
      rows = rows.filter((a) => Boolean(a.motif?.trim()) && !a.justifie);
    } else if (filtreJustif === "sans_motif") {
      rows = rows.filter((a) => !a.motif?.trim());
    }
    // Priorité : justificatif famille en tête
    rows.sort((a, b) => {
      const aPri = a.motif?.trim() && !a.justifie ? 0 : 1;
      const bPri = b.motif?.trim() && !b.justifie ? 0 : 1;
      if (aPri !== bPri) return aPri - bPri;
      return String(b.dateDebut).localeCompare(String(a.dateDebut));
    });
    return rows;
  }, [absences, filtreJustif]);

  const patch = async (
    id: string,
    body: { statut?: string; justifie?: boolean; motif?: string; noteCpe?: string },
  ) => {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/vie-scolaire/absences", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id, ...body }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Mise à jour impossible");
      setMessage("Absence mise à jour.");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const relancer = async (ids: string[]) => {
    setBusy(true);
    try {
      const res = await fetch("/api/vie-scolaire/absences", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "relance", ids }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Relance impossible");
      setMessage(`${data.updated} relance(s) marquée(s).`);
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className={embedded ? "space-y-5" : "max-w-3xl mx-auto px-4 py-6 space-y-5"}>
      <header className="flex flex-wrap items-end justify-between gap-3">
        {!embedded ? (
        <div>
          <h1 className="text-2xl font-black text-slate-900">Absences élèves</h1>
          <p className="text-sm text-slate-600 mt-1">
            Suivi CPE — justificatifs familles et relances après les appels.
          </p>
        </div>
        ) : (
          <div>
            <h2 className="text-lg font-black text-slate-900">Suivi des absences</h2>
            <p className="text-sm text-slate-600 mt-0.5">Justificatifs familles et relances CPE.</p>
          </div>
        )}
        <div className="flex flex-wrap items-center gap-2">
          <span className="rounded-full bg-amber-50 text-amber-900 px-3 py-1 text-sm font-bold">
            {aTraiter} à traiter
          </span>
          {withParentJustif.length > 0 ? (
            <span className="rounded-full bg-indigo-50 text-indigo-900 px-3 py-1 text-sm font-bold">
              {withParentJustif.length} justificatif(s) famille
            </span>
          ) : null}
          <a
            href="/api/vie-scolaire/absences/pdf?statut=a_traiter"
            className="rounded-xl border border-slate-200 bg-white px-3 py-1.5 text-xs font-bold text-slate-700 hover:bg-slate-50"
          >
            Export PDF
          </a>
          {!embedded ? (
          <Link
            href="/vie-scolaire/presence?tab=appel"
            className="rounded-xl border border-slate-200 bg-white px-3 py-1.5 text-xs font-bold text-slate-700 hover:bg-slate-50"
          >
            Appel
          </Link>
          ) : null}
        </div>
      </header>

      <div className="flex flex-wrap gap-2">
        {(
          [
            ["tous", "Tous"],
            ["justif_famille", "Justificatif famille"],
            ["sans_motif", "Sans motif"],
          ] as const
        ).map(([value, label]) => (
          <button
            key={value}
            type="button"
            onClick={() => setFiltreJustif(value)}
            className={`rounded-lg px-3 py-1.5 text-xs font-bold border ${
              filtreJustif === value
                ? "bg-indigo-600 text-white border-indigo-600"
                : "bg-white text-slate-700 border-slate-200"
            }`}
          >
            {label}
          </button>
        ))}
      </div>

      {visible.length > 0 && (
        <button
          type="button"
          disabled={busy}
          onClick={() => void relancer(visible.map((a) => a.id))}
          className="rounded-xl border border-slate-200 bg-white px-4 py-2 text-sm font-bold disabled:opacity-50"
        >
          Marquer la sélection en relance
        </button>
      )}

      {busy && !absences.length && <p className="text-sm text-slate-500">Chargement…</p>}

      {!busy && !absences.length && !error && (
        <p className="rounded-2xl border border-slate-200 bg-white p-4 text-sm text-slate-600">
          Aucune absence à traiter. Elles apparaîtront dès qu&apos;un professeur clôture un appel
          avec absents ou retards.
        </p>
      )}

      {!busy && absences.length > 0 && visible.length === 0 && (
        <p className="rounded-2xl border border-slate-200 bg-white p-4 text-sm text-slate-600">
          Aucune absence pour ce filtre.
        </p>
      )}

      <ul className="space-y-3">
        {visible.map((a) => {
          const parentJustif = Boolean(a.motif?.trim()) && !a.justifie;
          return (
            <li
              key={a.id}
              className={`rounded-2xl border p-4 space-y-3 ${
                parentJustif
                  ? "border-indigo-200 bg-indigo-50/40"
                  : "border-slate-200 bg-white"
              }`}
            >
              <div className="flex flex-wrap justify-between gap-2">
                <div>
                  <div className="flex flex-wrap items-center gap-2">
                    <p className="font-bold text-slate-900">
                      {a.elevePrenom} {a.eleveNom}
                    </p>
                    {parentJustif ? (
                      <span className="rounded-full bg-indigo-600 text-white px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide">
                        Justificatif famille
                      </span>
                    ) : null}
                    {a.source === "accueil" ? (
                      <span className="rounded-full bg-emerald-600 text-white px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide">
                        Prévenu (accueil)
                      </span>
                    ) : null}
                  </div>
                  <p className="text-sm text-slate-600">
                    {a.eleveClasse || "—"} · {a.type} ·{" "}
                    {a.dateDebut ? new Date(a.dateDebut).toLocaleDateString("fr-FR") : "—"}
                    {a.relanceAt ? " · déjà relancé" : ""}
                    {a.interne ? (
                      <>
                        {" "}
                        ·{" "}
                        <a
                          href="/gestion-internat"
                          className="font-bold text-indigo-700 underline"
                          title="Élève aussi dans le roster internat"
                        >
                          Interne
                        </a>
                      </>
                    ) : null}
                  </p>
                </div>
                <div className="flex flex-wrap gap-2">
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() =>
                      void patch(a.id, {
                        statut: "justifiee",
                        justifie: true,
                        motif: a.motif?.trim() || "Justifié",
                      })
                    }
                    className="rounded-lg bg-emerald-600 text-white px-3 py-1.5 text-xs font-bold disabled:opacity-50"
                  >
                    {parentJustif ? "Valider le justificatif" : "Justifier"}
                  </button>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void patch(a.id, { statut: "non_justifiee", justifie: false })}
                    className="rounded-lg bg-red-600 text-white px-3 py-1.5 text-xs font-bold disabled:opacity-50"
                  >
                    Non justifiée
                  </button>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void relancer([a.id])}
                    className="rounded-lg border border-slate-200 bg-white px-3 py-1.5 text-xs font-bold disabled:opacity-50"
                  >
                    Relancer
                  </button>
                </div>
              </div>

              {a.motif?.trim() ? (
                <p className="text-sm rounded-xl bg-white/80 border border-indigo-100 px-3 py-2 text-slate-800">
                  <span className="font-semibold text-indigo-900">Motif famille : </span>
                  {a.motif}
                </p>
              ) : (
                <p className="text-xs text-slate-500">Aucun motif famille pour l’instant.</p>
              )}
            </li>
          );
        })}
      </ul>

      {message && (
        <p className="text-sm text-emerald-800 bg-emerald-50 border border-emerald-100 rounded-xl px-3 py-2">
          {message}
        </p>
      )}
      {error && <p className="text-sm text-red-600 font-semibold">{error}</p>}
    </div>
  );
}
