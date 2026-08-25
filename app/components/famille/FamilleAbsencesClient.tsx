"use client";

import { useCallback, useEffect, useState } from "react";
import FamilleNav from "@/app/components/famille/FamilleNav";

type Enfant = {
  id: string;
  nom: string;
  prenom: string;
  classe: string | null;
};

type Absence = {
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
  motifEnAttente?: boolean;
};

function statutLabel(a: Absence): string {
  if (a.justifie || a.statut === "justifiee") return "Justifiée";
  if (a.statut === "justif_recue" || a.motifEnAttente) return "Justificatif reçu";
  if (a.statut === "non_justifiee") return "Non justifiée";
  if (a.statut === "en_cours") return "En cours de traitement";
  return a.statut;
}

function canJustify(a: Absence): boolean {
  if (a.justifie || a.statut === "justifiee" || a.statut === "non_justifiee" || a.statut === "classee") {
    return false;
  }
  return a.statut === "en_cours" || a.statut === "justif_recue" || Boolean(a.motifEnAttente);
}

export default function FamilleAbsencesClient() {
  const [enfants, setEnfants] = useState<Enfant[]>([]);
  const [absences, setAbsences] = useState<Absence[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [drafts, setDrafts] = useState<Record<string, string>>({});

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/famille/absences", { cache: "no-store" });
      const data = (await res.json()) as {
        error?: string;
        enfants?: Enfant[];
        absences?: Absence[];
      };
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setEnfants(data.enfants || []);
      setAbsences(data.absences || []);
      const next: Record<string, string> = {};
      for (const a of data.absences || []) {
        if (a.motif) next[a.id] = a.motif;
      }
      setDrafts((prev) => ({ ...next, ...prev }));
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const submitJustify = async (absenceId: string) => {
    setBusyId(absenceId);
    setMessage(null);
    setError(null);
    try {
      const res = await fetch("/api/famille/absences", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "justify",
          absenceId,
          motif: drafts[absenceId] || "",
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Envoi impossible");
      setMessage("Justificatif envoyé — la vie scolaire le traitera.");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusyId(null);
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-b from-indigo-50 to-white">
      <header className="border-b border-indigo-100 bg-white/90 backdrop-blur px-4 py-4">
        <div className="max-w-2xl mx-auto">
          <p className="text-xs font-bold uppercase tracking-wide text-indigo-600">Espace famille</p>
          <h1 className="text-2xl font-black text-slate-900 mt-0.5">Absences & retards</h1>
          <p className="text-sm text-slate-600 mt-1">
            Consultez et justifiez les absences signalées par l&apos;établissement.
          </p>
          <FamilleNav />
        </div>
      </header>

      <main className="max-w-2xl mx-auto px-4 py-6 space-y-6">
        {loading && <p className="text-sm text-slate-600">Chargement…</p>}

        {message && (
          <div className="rounded-2xl border border-emerald-200 bg-emerald-50 p-4 text-emerald-900 text-sm">
            {message}
          </div>
        )}

        {error && (
          <div className="rounded-2xl border border-red-200 bg-red-50 p-4 text-red-800 text-sm">
            {error}
            {error.includes("Non autorisé") || error.includes("AUTH") ? (
              <p className="mt-2">
                <a href="/auth/sign-in" className="font-bold underline">
                  Se connecter
                </a>
              </p>
            ) : null}
          </div>
        )}

        {!loading && !error && enfants.length > 0 && (
          <section className="rounded-2xl border border-slate-200 bg-white p-4">
            <h2 className="font-bold text-slate-900">Vos enfants</h2>
            <ul className="mt-3 space-y-2">
              {enfants.map((e) => (
                <li
                  key={e.id}
                  className="text-sm flex justify-between gap-2 border-b border-slate-100 pb-2"
                >
                  <span className="font-semibold">
                    {e.prenom} {e.nom}
                  </span>
                  <span className="text-slate-500">{e.classe || "—"}</span>
                </li>
              ))}
            </ul>
          </section>
        )}

        {!loading && !error && absences.length === 0 && (
          <p className="text-sm text-slate-600 rounded-2xl border border-slate-200 bg-white p-4">
            Aucune absence enregistrée pour le moment.
          </p>
        )}

        {!loading && absences.length > 0 && (
          <ul className="space-y-3">
            {absences.map((a) => (
              <li
                key={a.id}
                className="rounded-2xl border border-slate-200 bg-white p-4 text-sm space-y-2"
              >
                <div className="flex flex-wrap justify-between gap-2">
                  <p className="font-bold text-slate-900">
                    {a.elevePrenom} {a.eleveNom}
                  </p>
                  <span
                    className={`rounded-full px-2.5 py-0.5 text-xs font-bold ${
                      a.justifie || a.statut === "justifiee"
                        ? "bg-emerald-50 text-emerald-800"
                        : a.statut === "non_justifiee"
                          ? "bg-red-50 text-red-800"
                          : a.statut === "justif_recue" || a.motifEnAttente
                            ? "bg-indigo-50 text-indigo-800"
                            : "bg-amber-50 text-amber-900"
                    }`}
                  >
                    {statutLabel(a)}
                  </span>
                </div>
                <p className="text-slate-600">
                  {a.type} ·{" "}
                  {a.dateDebut ? new Date(a.dateDebut).toLocaleDateString("fr-FR") : "—"}
                  {a.eleveClasse ? ` · ${a.eleveClasse}` : ""}
                </p>

                {canJustify(a) ? (
                  <div className="space-y-2 pt-1">
                    <label className="block text-xs font-semibold text-slate-600">
                      Motif / justificatif
                      <textarea
                        className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
                        rows={2}
                        maxLength={500}
                        placeholder="Ex. rendez-vous médical, maladie…"
                        value={drafts[a.id] ?? ""}
                        onChange={(e) =>
                          setDrafts((prev) => ({ ...prev, [a.id]: e.target.value }))
                        }
                      />
                    </label>
                    <button
                      type="button"
                      disabled={busyId === a.id}
                      onClick={() => void submitJustify(a.id)}
                      className="rounded-xl bg-indigo-600 text-white px-3 py-2 text-xs font-bold disabled:opacity-50"
                    >
                      {busyId === a.id
                        ? "Envoi…"
                        : a.motifEnAttente || a.statut === "justif_recue"
                          ? "Mettre à jour le justificatif"
                          : "Envoyer le justificatif"}
                    </button>
                  </div>
                ) : a.motif ? (
                  <p className="text-xs text-slate-500">Motif : {a.motif}</p>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </main>
    </div>
  );
}
