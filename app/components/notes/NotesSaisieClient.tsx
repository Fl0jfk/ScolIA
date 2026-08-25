"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

type Matiere = { id: string; code: string; libelle: string };
type Periode = { id: string; code: string; libelle: string; statut: string };
type Devoir = {
  id: string;
  libelle: string;
  classe: string;
  dateDevoir: string | null;
  coefficient: string;
  matiereLibelle: string;
};
type Eleve = { id: string; nom: string; prenom: string };
type NoteRow = {
  devoirId: string;
  eleveId: string;
  valeur: string | null;
  absent: boolean;
  dispense: boolean;
};
type MoyenneRow = {
  eleveId: string;
  nom: string;
  prenom: string;
  matiereLibelle: string;
  moyenne: string | null;
  nbNotes: number;
};
type Groupe = { id: string; code: string; libelle: string; memberCount: number };

export default function NotesSaisieClient() {
  const [matieres, setMatieres] = useState<Matiere[]>([]);
  const [periodes, setPeriodes] = useState<Periode[]>([]);
  const [groupes, setGroupes] = useState<Groupe[]>([]);
  const [devoirs, setDevoirs] = useState<Devoir[]>([]);
  const [eleves, setEleves] = useState<Eleve[]>([]);
  const [notes, setNotes] = useState<NoteRow[]>([]);
  const [moyennes, setMoyennes] = useState<MoyenneRow[]>([]);
  const [scopeMode, setScopeMode] = useState<"classe" | "groupe">("classe");
  const [classe, setClasse] = useState("");
  const [groupeId, setGroupeId] = useState("");
  const [periodeId, setPeriodeId] = useState("");
  const [matiereId, setMatiereId] = useState("");
  const [selectedDevoirId, setSelectedDevoirId] = useState("");
  const [view, setView] = useState<"saisie" | "moyennes">("saisie");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [devoirForm, setDevoirForm] = useState({
    libelle: "",
    dateDevoir: "",
    coefficient: "1",
  });

  const noteMap = useMemo(() => {
    const m = new Map<string, NoteRow>();
    for (const n of notes) m.set(`${n.devoirId}:${n.eleveId}`, n);
    return m;
  }, [notes]);

  const loadGrid = useCallback(async () => {
    setError(null);
    const qs = new URLSearchParams();
    if (scopeMode === "groupe" && groupeId) qs.set("groupeId", groupeId);
    else if (classe.trim()) qs.set("classe", classe.trim());
    if (periodeId) qs.set("periodeId", periodeId);
    if (matiereId) qs.set("matiereId", matiereId);
    if (selectedDevoirId) qs.set("devoirId", selectedDevoirId);
    if (view === "moyennes") qs.set("view", "moyennes");

    const res = await fetch(`/api/notes/saisie?${qs}`, { cache: "no-store" });
    const data = await res.json();
    if (!res.ok) throw new Error(data?.error || "Chargement impossible");

    setMatieres(data.matieres || []);
    setPeriodes(data.periodes || []);
    setGroupes(data.groupes || []);
    setDevoirs(data.devoirs || []);
    setEleves(data.eleves || []);
    setNotes(data.notes || []);
    setMoyennes(data.moyennes || []);
  }, [scopeMode, classe, groupeId, periodeId, matiereId, selectedDevoirId, view]);

  useEffect(() => {
    void loadGrid().catch((e: unknown) => {
      setError(e instanceof Error ? e.message : "Erreur");
    });
  }, [loadGrid]);

  const post = async (body: Record<string, unknown>) => {
    setBusy(true);
    setMessage(null);
    setError(null);
    try {
      const res = await fetch("/api/notes/saisie", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Échec");
      await loadGrid();
      return data;
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
      return null;
    } finally {
      setBusy(false);
    }
  };

  const selectedPeriode = periodes.find((p) => p.id === periodeId);
  const periodeCloturee = selectedPeriode?.statut === "cloturee";
  const scopeReady = scopeMode === "groupe" ? Boolean(groupeId) : Boolean(classe.trim());

  return (
    <ModulePageShell maxWidthClass="max-w-6xl">
      <ModulePageHeader
        eyebrow="Administratif · Notes Phase 2"
        title="Saisie des notes"
        description="Devoirs par classe ou groupe pédagogique (LV2, options), moyennes pondérées et clôture de période."
        actions={
          <div className="flex flex-wrap gap-3">
            <Link
              href="/groupes-pedagogiques"
              className="text-sm font-bold text-indigo-600 hover:underline"
            >
              Groupes →
            </Link>
            <Link href="/notes/competences" className="text-sm font-bold text-indigo-600 hover:underline">
              Compétences LSU →
            </Link>
            <Link href="/notes/bulletins" className="text-sm font-bold text-indigo-600 hover:underline">
              Bulletins PDF →
            </Link>
            <Link href="/notes/espace" className="text-sm font-bold text-indigo-600 hover:underline">
              ← Configuration
            </Link>
          </div>
        }
      />

      <div className="flex flex-wrap gap-3 mb-4 items-end">
        <div className="flex rounded-xl border border-slate-200 overflow-hidden text-sm font-bold">
          <button
            type="button"
            onClick={() => {
              setScopeMode("classe");
              setGroupeId("");
              setSelectedDevoirId("");
            }}
            className={`px-3 py-2 ${scopeMode === "classe" ? "bg-indigo-600 text-white" : "bg-white text-slate-700"}`}
          >
            Classe
          </button>
          <button
            type="button"
            onClick={() => {
              setScopeMode("groupe");
              setClasse("");
              setSelectedDevoirId("");
            }}
            className={`px-3 py-2 ${scopeMode === "groupe" ? "bg-indigo-600 text-white" : "bg-white text-slate-700"}`}
          >
            Groupe
          </button>
        </div>
        {scopeMode === "classe" ? (
          <input
            className="border rounded-xl px-3 py-2 text-sm w-28"
            placeholder="Classe (3e2)"
            value={classe}
            onChange={(e) => setClasse(e.target.value)}
          />
        ) : (
          <select
            className="border rounded-xl px-3 py-2 text-sm min-w-[12rem]"
            value={groupeId}
            onChange={(e) => setGroupeId(e.target.value)}
          >
            <option value="">Groupe pédagogique</option>
            {groupes.map((g) => (
              <option key={g.id} value={g.id}>
                {g.code} — {g.libelle} ({g.memberCount})
              </option>
            ))}
          </select>
        )}
        <select
          className="border rounded-xl px-3 py-2 text-sm"
          value={periodeId}
          onChange={(e) => setPeriodeId(e.target.value)}
        >
          <option value="">Période</option>
          {periodes.map((p) => (
            <option key={p.id} value={p.id}>
              {p.libelle} ({p.statut})
            </option>
          ))}
        </select>
        <select
          className="border rounded-xl px-3 py-2 text-sm"
          value={matiereId}
          onChange={(e) => setMatiereId(e.target.value)}
        >
          <option value="">Matière</option>
          {matieres.map((m) => (
            <option key={m.id} value={m.id}>
              {m.code} — {m.libelle}
            </option>
          ))}
        </select>
        <button
          type="button"
          onClick={() => setView(view === "saisie" ? "moyennes" : "saisie")}
          className="rounded-xl border border-slate-200 px-3 py-2 text-sm font-bold"
        >
          {view === "saisie" ? "Voir moyennes" : "Voir saisie"}
        </button>
        {periodeId && !periodeCloturee && (
          <button
            type="button"
            disabled={busy}
            onClick={async () => {
              if (!confirm("Clôturer cette période ? La saisie sera bloquée.")) return;
              await post({ action: "closePeriode", periodeId });
              setMessage("Période clôturée.");
            }}
            className="rounded-xl bg-amber-600 text-white px-3 py-2 text-sm font-bold disabled:opacity-50"
          >
            Clôturer période
          </button>
        )}
      </div>

      {message && <p className="mb-3 text-sm text-emerald-700 font-semibold">{message}</p>}
      {error && <p className="mb-3 text-sm text-red-600">{error}</p>}
      {periodeCloturee && (
        <p className="mb-3 text-sm text-amber-800 bg-amber-50 rounded-xl px-3 py-2">
          Période clôturée — consultation seule.
        </p>
      )}

      {view === "moyennes" ? (
        <section className="rounded-2xl border border-slate-200 bg-white p-5 overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-slate-500">
                <th className="py-2">Élève</th>
                <th>Matière</th>
                <th>Moyenne</th>
                <th>Nb notes</th>
              </tr>
            </thead>
            <tbody>
              {moyennes.map((m) => (
                <tr key={`${m.eleveId}-${m.matiereLibelle}`} className="border-t border-slate-100">
                  <td className="py-2">
                    {m.nom} {m.prenom}
                  </td>
                  <td>{m.matiereLibelle}</td>
                  <td className="font-semibold">{m.moyenne ?? "—"}</td>
                  <td>{m.nbNotes}</td>
                </tr>
              ))}
              {!moyennes.length && (
                <tr>
                  <td colSpan={4} className="py-4 text-slate-500">
                    Renseignez {scopeMode === "groupe" ? "un groupe" : "une classe"} + période — saisissez
                    des notes pour calculer les moyennes.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </section>
      ) : (
        <div className="grid gap-6 lg:grid-cols-3">
          <section className="lg:col-span-1 rounded-2xl border border-slate-200 bg-white p-5 space-y-3">
            <h2 className="font-black text-slate-900">Devoirs</h2>
            {!periodeCloturee && scopeReady && matiereId && periodeId && (
              <div className="space-y-2 text-sm">
                <input
                  className="border rounded-xl px-3 py-2 w-full"
                  placeholder="Libellé devoir"
                  value={devoirForm.libelle}
                  onChange={(e) => setDevoirForm({ ...devoirForm, libelle: e.target.value })}
                />
                <div className="flex gap-2">
                  <input
                    type="date"
                    className="border rounded-xl px-2 py-2 flex-1"
                    value={devoirForm.dateDevoir}
                    onChange={(e) => setDevoirForm({ ...devoirForm, dateDevoir: e.target.value })}
                  />
                  <input
                    className="border rounded-xl px-2 py-2 w-16"
                    placeholder="Coef"
                    value={devoirForm.coefficient}
                    onChange={(e) => setDevoirForm({ ...devoirForm, coefficient: e.target.value })}
                  />
                </div>
                <button
                  type="button"
                  disabled={busy}
                  onClick={async () => {
                    const payload: Record<string, unknown> = {
                      action: "createDevoir",
                      ...devoirForm,
                      matiereId,
                      periodeId,
                    };
                    if (scopeMode === "groupe") payload.groupeId = groupeId;
                    else payload.classe = classe.trim();
                    const data = await post(payload);
                    if (data?.devoir?.id) {
                      setSelectedDevoirId(data.devoir.id);
                      setDevoirForm({ libelle: "", dateDevoir: "", coefficient: "1" });
                      setMessage("Devoir créé.");
                    }
                  }}
                  className="w-full rounded-xl bg-indigo-600 text-white py-2 font-bold disabled:opacity-50"
                >
                  + Devoir
                </button>
              </div>
            )}
            <ul className="space-y-1 max-h-80 overflow-y-auto text-sm">
              {devoirs.map((d) => (
                <li key={d.id}>
                  <button
                    type="button"
                    onClick={() => setSelectedDevoirId(d.id)}
                    className={`w-full text-left rounded-lg px-2 py-2 ${
                      selectedDevoirId === d.id ? "bg-indigo-50 border border-indigo-200" : "hover:bg-slate-50"
                    }`}
                  >
                    <span className="font-semibold">{d.libelle}</span>
                    <span className="block text-xs text-slate-500">
                      {d.matiereLibelle} · coef {d.coefficient}
                    </span>
                  </button>
                </li>
              ))}
              {!devoirs.length && (
                <li className="text-slate-500">
                  Filtrez {scopeMode === "groupe" ? "groupe" : "classe"} / période / matière.
                </li>
              )}
            </ul>
          </section>

          <section className="lg:col-span-2 rounded-2xl border border-slate-200 bg-white p-5 overflow-x-auto">
            <h2 className="font-black text-slate-900 mb-3">Grille de saisie</h2>
            {selectedDevoirId && eleves.length ? (
              <table className="w-full text-sm">
                <thead>
                  <tr className="text-left text-slate-500">
                    <th className="py-2">Élève</th>
                    <th>Note /20</th>
                    <th>Abs.</th>
                    <th>Disp.</th>
                  </tr>
                </thead>
                <tbody>
                  {eleves.map((el) => {
                    const n = noteMap.get(`${selectedDevoirId}:${el.id}`);
                    return (
                      <tr key={el.id} className="border-t border-slate-100">
                        <td className="py-2">
                          {el.nom} {el.prenom}
                        </td>
                        <td>
                          <input
                            className="border rounded-lg px-2 py-1 w-20"
                            defaultValue={n?.valeur ?? ""}
                            disabled={periodeCloturee || busy}
                            onBlur={(e) => {
                              if (periodeCloturee) return;
                              void post({
                                action: "upsertNote",
                                devoirId: selectedDevoirId,
                                eleveId: el.id,
                                valeur: e.target.value,
                                absent: n?.absent ?? false,
                                dispense: n?.dispense ?? false,
                              });
                            }}
                          />
                        </td>
                        <td>
                          <input
                            type="checkbox"
                            defaultChecked={n?.absent ?? false}
                            disabled={periodeCloturee || busy}
                            onChange={(e) => {
                              void post({
                                action: "upsertNote",
                                devoirId: selectedDevoirId,
                                eleveId: el.id,
                                valeur: n?.valeur ?? null,
                                absent: e.target.checked,
                                dispense: n?.dispense ?? false,
                              });
                            }}
                          />
                        </td>
                        <td>
                          <input
                            type="checkbox"
                            defaultChecked={n?.dispense ?? false}
                            disabled={periodeCloturee || busy}
                            onChange={(e) => {
                              void post({
                                action: "upsertNote",
                                devoirId: selectedDevoirId,
                                eleveId: el.id,
                                valeur: n?.valeur ?? null,
                                absent: n?.absent ?? false,
                                dispense: e.target.checked,
                              });
                            }}
                          />
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            ) : (
              <p className="text-slate-500 text-sm">
                Sélectionnez un devoir — les élèves du{" "}
                {scopeMode === "groupe" ? "groupe" : "classe"} s’affichent ici.
              </p>
            )}
          </section>
        </div>
      )}
    </ModulePageShell>
  );
}
