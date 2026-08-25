"use client";

import Link from "next/link";
import { useCallback, useEffect, useMemo, useState } from "react";

type EleveRow = {
  id: string;
  nom: string;
  prenom: string;
  photoKey: string | null;
  photoUrl?: string | null;
  classe: string | null;
};

type CreneauRow = {
  id: string;
  heureDebut: string;
  heureFin: string;
  classe: string | null;
  groupeId?: string | null;
  groupeCode?: string | null;
  matiereLibelle: string | null;
  enseignantNom: string | null;
  salle: string | null;
};

type AppelResume = {
  id: string;
  classe: string;
  creneauId: string | null;
  statut: string;
  heureDebut: string | null;
  matiereLibelle: string | null;
};

type LigneStatut = "present" | "absent" | "retard" | "dispense";

type LigneState = {
  eleveId: string;
  statut: LigneStatut;
  retardMinutes: number | null;
};

function todayIso(): string {
  const d = new Date();
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

function initials(prenom: string, nom: string): string {
  return `${prenom.slice(0, 1)}${nom.slice(0, 1)}`.toUpperCase();
}

export default function VsAppelsClient() {
  const [classe, setClasse] = useState("");
  const [dateAppel, setDateAppel] = useState(todayIso());
  const [creneaux, setCreneaux] = useState<CreneauRow[]>([]);
  const [appelsDuJour, setAppelsDuJour] = useState<AppelResume[]>([]);
  const [manquants, setManquants] = useState<number>(0);
  const [appelId, setAppelId] = useState<string | null>(null);
  const [eleves, setEleves] = useState<EleveRow[]>([]);
  const [lignes, setLignes] = useState<Record<string, LigneState>>({});
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [clos, setClos] = useState(false);

  const counts = useMemo(() => {
    const values = Object.values(lignes);
    return {
      present: values.filter((l) => l.statut === "present").length,
      absent: values.filter((l) => l.statut === "absent").length,
      retard: values.filter((l) => l.statut === "retard").length,
      dispense: values.filter((l) => l.statut === "dispense").length,
    };
  }, [lignes]);

  const loadCreneaux = useCallback(async () => {
    setError(null);
    try {
      const params = new URLSearchParams({ date: dateAppel });
      if (classe.trim()) params.set("classe", classe.trim());
      const res = await fetch(`/api/vie-scolaire/appels?${params}`, { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement EDT impossible");
      setCreneaux(data.creneaux || []);
      setAppelsDuJour(data.appels || []);
      setManquants(Array.isArray(data.manquants) ? data.manquants.length : 0);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    }
  }, [dateAppel, classe]);

  useEffect(() => {
    void loadCreneaux();
  }, [loadCreneaux]);

  const applyAppelPayload = (data: {
    appel: { id: string; statut: string };
    eleves: EleveRow[];
    lignes: Array<{ eleveId: string; statut: string; retardMinutes?: number | null }>;
  }) => {
    setAppelId(data.appel.id);
    setClos(data.appel.statut === "clos");
    setEleves(data.eleves || []);
    const next: Record<string, LigneState> = {};
    for (const e of data.eleves || []) {
      const existing = (data.lignes || []).find((l) => l.eleveId === e.id);
      next[e.id] = {
        eleveId: e.id,
        statut: (existing?.statut as LigneStatut) || "present",
        retardMinutes: existing?.retardMinutes ?? null,
      };
    }
    setLignes(next);
  };

  const startAppel = async (fromCreneau?: CreneauRow) => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const creneau = fromCreneau;
      const classeCible = (creneau?.classe || creneau?.groupeCode || classe).trim();
      if (!classeCible && !creneau?.groupeId) {
        throw new Error("Choisissez un créneau EDT ou saisissez une classe.");
      }

      const res = await fetch("/api/vie-scolaire/appels", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "create",
          classe: classeCible,
          dateAppel,
          creneauId: creneau?.id || null,
          heureDebut: creneau?.heureDebut || null,
          heureFin: creneau?.heureFin || null,
          matiereLibelle: creneau?.matiereLibelle || null,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Impossible de démarrer l'appel");
      applyAppelPayload(data);
      setClasse(classeCible);
      setMessage(
        `${(data.eleves || []).length} élève(s)` +
          (creneau
            ? ` — ${creneau.heureDebut}–${creneau.heureFin}${creneau.matiereLibelle ? ` · ${creneau.matiereLibelle}` : ""}`
            : "") +
          ".",
      );
      await loadCreneaux();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const setStatut = useCallback((eleveId: string, statut: LigneStatut) => {
    setLignes((prev) => ({
      ...prev,
      [eleveId]: {
        eleveId,
        statut,
        retardMinutes: statut === "retard" ? prev[eleveId]?.retardMinutes ?? 5 : null,
      },
    }));
  }, []);

  const save = async (closeAfter = false) => {
    if (!appelId) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/vie-scolaire/appels", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "save",
          appelId,
          lignes: Object.values(lignes),
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Enregistrement impossible");

      if (closeAfter) {
        const closeRes = await fetch("/api/vie-scolaire/appels", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ action: "close", appelId }),
        });
        const closeData = await closeRes.json();
        if (!closeRes.ok) throw new Error(closeData.error || "Clôture impossible");
        setClos(true);
        setMessage("Appel enregistré et clos. Absents transmis au CPE (et signalés si internes).");
      } else {
        setMessage(`Enregistré (${data.saved} ligne(s)).`);
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const reopenPicker = () => {
    setAppelId(null);
    setEleves([]);
    setLignes({});
    setClos(false);
    setMessage(null);
    void loadCreneaux();
  };

  return (
    <div className="max-w-2xl mx-auto px-4 py-6 space-y-5">
      <header>
        <h1 className="text-2xl font-black text-slate-900">Appel de classe</h1>
        <p className="text-sm text-slate-600 mt-1">
          Choisissez un créneau EDT du jour, puis cochez absents et retards. Photos si disponibles.
        </p>
        {manquants > 0 && !appelId && (
          <p className="mt-2 text-sm font-semibold text-amber-900 bg-amber-50 border border-amber-100 rounded-xl px-3 py-2">
            {manquants} créneau(x) déjà commencé(s) sans appel clôturé.
          </p>
        )}
        <div className="mt-3 flex flex-wrap gap-2">
          <Link
            href="/vie-scolaire/calendrier"
            className="rounded-lg border border-slate-200 px-2.5 py-1 text-xs font-bold text-slate-700 hover:bg-slate-50"
          >
            EDT & calendrier
          </Link>
          <Link
            href="/vie-scolaire/absences"
            className="rounded-lg border border-slate-200 px-2.5 py-1 text-xs font-bold text-slate-700 hover:bg-slate-50"
          >
            Absences
          </Link>
        </div>
      </header>

      {!appelId && (
        <section className="rounded-2xl border border-slate-200 bg-white p-4 space-y-3">
          <div className="grid gap-3 sm:grid-cols-2">
            <label className="block text-sm font-semibold">
              Date
              <input
                type="date"
                value={dateAppel}
                onChange={(e) => setDateAppel(e.target.value)}
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
              />
            </label>
            <label className="block text-sm font-semibold">
              Filtrer classe (optionnel)
              <input
                value={classe}
                onChange={(e) => setClasse(e.target.value)}
                placeholder="ex. 3E1"
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
              />
            </label>
          </div>

          {creneaux.length > 0 ? (
            <ul className="space-y-2 max-h-64 overflow-y-auto">
              {creneaux.map((c) => {
                const deja = appelsDuJour.find((a) => a.creneauId === c.id);
                return (
                  <li key={c.id}>
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() => void startAppel(c)}
                      className="w-full text-left rounded-xl border border-slate-200 hover:border-indigo-300 hover:bg-indigo-50 px-3 py-2.5 disabled:opacity-50"
                    >
                      <div className="flex justify-between gap-2 text-sm">
                        <span className="font-bold text-slate-900">
                          {c.heureDebut}–{c.heureFin}
                          {c.groupeCode ? (
                            <span className="text-indigo-700"> · grp {c.groupeCode}</span>
                          ) : c.classe ? (
                            ` · ${c.classe}`
                          ) : (
                            ""
                          )}
                        </span>
                        {deja && (
                          <span
                            className={`text-xs font-semibold ${
                              deja.statut === "clos" ? "text-slate-500" : "text-amber-700"
                            }`}
                          >
                            {deja.statut === "clos" ? "Clos" : "En cours"}
                          </span>
                        )}
                      </div>
                      <p className="text-xs text-slate-600 mt-0.5">
                        {c.matiereLibelle || "Matière"}
                        {c.salle ? ` · salle ${c.salle}` : ""}
                        {c.enseignantNom ? ` · ${c.enseignantNom}` : ""}
                      </p>
                    </button>
                  </li>
                );
              })}
            </ul>
          ) : (
            <p className="text-xs text-slate-500">
              Aucun créneau EDT pour ce jour. Saisissez une classe et démarrez un appel libre, ou
              renseignez l&apos;EDT dans Calendrier & EDT.
            </p>
          )}

          <div className="flex flex-wrap gap-2 items-end border-t border-slate-100 pt-3">
            <label className="block text-sm font-semibold flex-1 min-w-[8rem]">
              Classe (appel libre)
              <input
                value={classe}
                onChange={(e) => setClasse(e.target.value)}
                placeholder="ex. 3E1"
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
              />
            </label>
            <button
              type="button"
              disabled={busy || !classe.trim()}
              onClick={() => void startAppel()}
              className="rounded-xl bg-indigo-600 text-white px-4 py-2 font-bold disabled:opacity-50"
            >
              {busy ? "…" : "Appel libre"}
            </button>
          </div>
        </section>
      )}

      {appelId && (
        <>
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="flex flex-wrap gap-2 text-xs font-semibold">
              <span className="rounded-full bg-emerald-50 text-emerald-800 px-3 py-1">
                Présents {counts.present}
              </span>
              <span className="rounded-full bg-red-50 text-red-800 px-3 py-1">
                Absents {counts.absent}
              </span>
              <span className="rounded-full bg-amber-50 text-amber-800 px-3 py-1">
                Retards {counts.retard}
              </span>
            </div>
            <button
              type="button"
              onClick={reopenPicker}
              className="text-xs font-bold text-indigo-700 underline"
            >
              Changer de créneau
            </button>
          </div>

          <ul className="space-y-2">
            {eleves.map((e) => {
              const ligne = lignes[e.id];
              const statut = ligne?.statut || "present";
              return (
                <li
                  key={e.id}
                  className="rounded-2xl border border-slate-200 bg-white p-3 flex flex-wrap items-center gap-3 justify-between"
                >
                  <div className="flex items-center gap-3 min-w-0">
                    {e.photoUrl ? (
                      // eslint-disable-next-line @next/next/no-img-element
                      <img
                        src={e.photoUrl}
                        alt=""
                        className="h-11 w-11 rounded-full object-cover shrink-0 bg-slate-100"
                      />
                    ) : (
                      <div className="h-11 w-11 rounded-full bg-slate-100 flex items-center justify-center text-xs font-bold text-slate-600 shrink-0">
                        {initials(e.prenom, e.nom)}
                      </div>
                    )}
                    <div className="min-w-0">
                      <p className="font-bold text-slate-900 truncate">
                        {e.prenom} {e.nom}
                      </p>
                    </div>
                  </div>
                  <div className="flex flex-wrap gap-1">
                    {(
                      [
                        ["present", "Présent"],
                        ["absent", "Absent"],
                        ["retard", "Retard"],
                        ["dispense", "Disp."],
                      ] as const
                    ).map(([value, label]) => (
                      <button
                        key={value}
                        type="button"
                        disabled={clos || busy}
                        onClick={() => setStatut(e.id, value)}
                        className={`rounded-lg px-2.5 py-1.5 text-xs font-bold border ${
                          statut === value
                            ? value === "absent"
                              ? "bg-red-600 text-white border-red-600"
                              : value === "retard"
                                ? "bg-amber-500 text-white border-amber-500"
                                : value === "dispense"
                                  ? "bg-slate-600 text-white border-slate-600"
                                  : "bg-emerald-600 text-white border-emerald-600"
                            : "bg-white text-slate-600 border-slate-200"
                        }`}
                      >
                        {label}
                      </button>
                    ))}
                  </div>
                </li>
              );
            })}
          </ul>

          {!clos && (
            <div className="flex flex-wrap gap-2 sticky bottom-4">
              <button
                type="button"
                disabled={busy}
                onClick={() => void save(false)}
                className="rounded-xl border border-indigo-200 bg-indigo-50 text-indigo-800 px-4 py-2 font-bold disabled:opacity-50"
              >
                Enregistrer
              </button>
              <button
                type="button"
                disabled={busy}
                onClick={() => void save(true)}
                className="rounded-xl bg-indigo-600 text-white px-4 py-2 font-bold disabled:opacity-50"
              >
                Enregistrer et clôturer
              </button>
            </div>
          )}
        </>
      )}

      {message && (
        <p className="text-sm text-emerald-800 bg-emerald-50 border border-emerald-100 rounded-xl px-3 py-2">
          {message}
        </p>
      )}
      {error && <p className="text-sm text-red-600 font-semibold">{error}</p>}
    </div>
  );
}
