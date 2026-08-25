"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

type Groupe = {
  id: string;
  code: string;
  libelle: string;
  type: string;
  memberCount: number;
  anneeScolaireId?: string | null;
};

type Membre = {
  eleveId: string;
  nom: string;
  prenom: string;
  classe: string | null;
  ine: string | null;
};

const TYPES = [
  { value: "option", label: "Option" },
  { value: "lv2", label: "LV2" },
  { value: "sport", label: "Sport / APS" },
  { value: "internat", label: "Internat" },
  { value: "autre", label: "Autre" },
];

export default function GroupesPedagogiquesClient() {
  const [groupes, setGroupes] = useState<Groupe[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [membres, setMembres] = useState<Membre[]>([]);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [groupeForm, setGroupeForm] = useState({ code: "", libelle: "", type: "option" });
  const [search, setSearch] = useState("");
  const [searchHits, setSearchHits] = useState<Membre[]>([]);
  const [classeBulk, setClasseBulk] = useState("");
  const [anneeLabel, setAnneeLabel] = useState<string | null>(null);

  const loadGroupes = useCallback(async () => {
    setError(null);
    try {
      const res = await fetch("/api/groupes-pedagogiques", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Chargement impossible");
      setGroupes(data.groupes || []);
      if (data.anneeCourante?.label) setAnneeLabel(String(data.anneeCourante.label));
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    }
  }, []);

  const loadMembres = useCallback(async (groupeId: string) => {
    try {
      const res = await fetch(`/api/groupes-pedagogiques?groupeId=${encodeURIComponent(groupeId)}`, {
        cache: "no-store",
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Chargement membres impossible");
      setMembres(data.membres || []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    }
  }, []);

  useEffect(() => {
    void loadGroupes();
  }, [loadGroupes]);

  useEffect(() => {
    if (selectedId) void loadMembres(selectedId);
    else setMembres([]);
  }, [selectedId, loadMembres]);

  useEffect(() => {
    const q = search.trim();
    if (q.length < 2) {
      setSearchHits([]);
      return;
    }
    const t = setTimeout(() => {
      void fetch(`/api/groupes-pedagogiques?search=${encodeURIComponent(q)}`)
        .then((r) => r.json())
        .then((data) => setSearchHits(data.eleves || []))
        .catch(() => setSearchHits([]));
    }, 250);
    return () => clearTimeout(t);
  }, [search]);

  const post = async (body: Record<string, unknown>) => {
    setBusy(true);
    setMessage(null);
    setError(null);
    try {
      const res = await fetch("/api/groupes-pedagogiques", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Échec");
      await loadGroupes();
      if (selectedId) await loadMembres(selectedId);
      return data;
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
      return null;
    } finally {
      setBusy(false);
    }
  };

  const selected = groupes.find((g) => g.id === selectedId) ?? null;

  return (
    <ModulePageShell maxWidthClass="max-w-6xl">
      <ModulePageHeader
        eyebrow="Transversal Notes · Vie scolaire"
        title="Groupes pédagogiques"
        description={
          anneeLabel
            ? `Configuration manuelle unique — année ${anneeLabel}. Consommée par Notes, EDT, appels et dossier élève.`
            : "Configuration manuelle unique — consommée par Notes, EDT, appels et dossier élève."
        }
        actions={
          <div className="flex flex-wrap gap-3 text-sm font-bold">
            <Link href="/parametres?tab=annees" className="text-slate-600 hover:underline">
              Année scolaire
            </Link>
            <Link href="/notes/saisie" className="text-indigo-600 hover:underline">
              Saisie notes
            </Link>
            <Link href="/notes/espace" className="text-indigo-600 hover:underline">
              Config notes
            </Link>
            <Link href="/vie-scolaire/calendrier" className="text-indigo-600 hover:underline">
              EDT
            </Link>
            <Link href="/vie-scolaire/presence?tab=appel" className="text-indigo-600 hover:underline">
              Appels
            </Link>
            <Link href="/toolbox/repartition-classes" className="text-emerald-700 hover:underline">
              Composition classes
            </Link>
          </div>
        }
      />

      {message && <p className="mb-3 text-sm text-emerald-700 font-semibold">{message}</p>}
      {error && <p className="mb-3 text-sm text-red-600">{error}</p>}

      <div className="grid gap-6 lg:grid-cols-2">
        <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
          <h2 className="font-black text-slate-900">Groupes ({groupes.length})</h2>
          <div className="grid gap-2 text-sm">
            <div className="flex gap-2">
              <input
                className="border rounded-xl px-3 py-2 w-24"
                placeholder="Code"
                value={groupeForm.code}
                onChange={(e) => setGroupeForm({ ...groupeForm, code: e.target.value })}
              />
              <input
                className="border rounded-xl px-3 py-2 flex-1"
                placeholder="Libellé (ex. Allemand LV2 A)"
                value={groupeForm.libelle}
                onChange={(e) => setGroupeForm({ ...groupeForm, libelle: e.target.value })}
              />
            </div>
            <select
              className="border rounded-xl px-3 py-2"
              value={groupeForm.type}
              onChange={(e) => setGroupeForm({ ...groupeForm, type: e.target.value })}
            >
              {TYPES.map((t) => (
                <option key={t.value} value={t.value}>
                  {t.label}
                </option>
              ))}
            </select>
            <button
              type="button"
              disabled={busy}
              onClick={async () => {
                const data = await post({ action: "upsertGroupe", groupe: groupeForm });
                if (data?.groupe?.id) {
                  setSelectedId(data.groupe.id);
                  setMessage(`Groupe ${data.groupe.code} enregistré.`);
                  setGroupeForm({ code: "", libelle: "", type: "option" });
                }
              }}
              className="rounded-xl bg-indigo-600 text-white px-4 py-2 font-bold disabled:opacity-50"
            >
              Créer / mettre à jour
            </button>
          </div>

          <ul className="space-y-1 max-h-80 overflow-y-auto">
            {groupes.map((g) => (
              <li key={g.id}>
                <button
                  type="button"
                  onClick={() => setSelectedId(g.id)}
                  className={`w-full text-left rounded-xl px-3 py-2 text-sm flex justify-between gap-2 ${
                    selectedId === g.id ? "bg-indigo-50 border border-indigo-200" : "hover:bg-slate-50"
                  }`}
                >
                  <span>
                    <span className="font-mono font-bold">{g.code}</span> — {g.libelle}
                    <span className="text-slate-500 block text-xs">{g.type}</span>
                  </span>
                  <span className="font-semibold text-slate-600">{g.memberCount}</span>
                </button>
              </li>
            ))}
            {!groupes.length && <li className="text-slate-500 text-sm">Aucun groupe — créez-en un.</li>}
          </ul>
        </section>

        <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
          <h2 className="font-black text-slate-900">
            {selected ? `Membres — ${selected.code}` : "Sélectionnez un groupe"}
          </h2>

          {selected && (
            <>
              <div className="flex gap-2 text-sm">
                <input
                  className="border rounded-xl px-3 py-2 flex-1"
                  placeholder="Ajouter par classe (ex. 3e2)"
                  value={classeBulk}
                  onChange={(e) => setClasseBulk(e.target.value)}
                />
                <button
                  type="button"
                  disabled={busy || !classeBulk.trim()}
                  onClick={async () => {
                    const data = await post({
                      action: "addFromClasse",
                      groupeId: selected.id,
                      classe: classeBulk,
                    });
                    if (data) {
                      setMessage(
                        `${data.added} élève(s) ajouté(s) sur ${data.total} dans la classe « ${classeBulk} ».`,
                      );
                      setClasseBulk("");
                    }
                  }}
                  className="rounded-xl bg-slate-900 text-white px-3 py-2 font-bold disabled:opacity-50 whitespace-nowrap"
                >
                  + Classe
                </button>
                <button
                  type="button"
                  disabled={busy}
                  onClick={async () => {
                    if (!confirm(`Supprimer le groupe ${selected.code} ?`)) return;
                    await post({ action: "deleteGroupe", id: selected.id });
                    setSelectedId(null);
                    setMessage("Groupe supprimé.");
                  }}
                  className="rounded-xl border border-red-200 text-red-600 px-3 py-2 font-bold"
                >
                  Suppr.
                </button>
              </div>

              <div className="space-y-2">
                <input
                  className="border rounded-xl px-3 py-2 text-sm w-full"
                  placeholder="Rechercher un élève (nom, INE, classe…)"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                />
                {searchHits.length > 0 && (
                  <ul className="border rounded-xl divide-y max-h-32 overflow-y-auto text-sm">
                    {searchHits.map((e) => (
                      <li key={e.eleveId} className="flex justify-between items-center px-3 py-2">
                        <span>
                          {e.nom} {e.prenom}
                          <span className="text-slate-500 text-xs block">
                            {e.classe || "—"} · {e.ine || "sans INE"}
                          </span>
                        </span>
                        <button
                          type="button"
                          className="text-indigo-600 font-bold text-xs"
                          onClick={async () => {
                            const data = await post({
                              action: "addMembre",
                              groupeId: selected.id,
                              eleveId: e.eleveId,
                            });
                            if (data) {
                              setMessage(data.added ? "Élève ajouté." : "Élève déjà membre.");
                              setSearch("");
                              setSearchHits([]);
                            }
                          }}
                        >
                          Ajouter
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
              </div>

              <ul className="space-y-1 max-h-64 overflow-y-auto text-sm">
                {membres.map((m) => (
                  <li
                    key={m.eleveId}
                    className="flex justify-between items-center border-b border-slate-100 py-2"
                  >
                    <span>
                      {m.nom} {m.prenom}
                      <span className="text-slate-500 text-xs block">{m.classe || "—"}</span>
                    </span>
                    <button
                      type="button"
                      className="text-red-600 font-bold text-xs"
                      onClick={() =>
                        void post({
                          action: "removeMembre",
                          groupeId: selected.id,
                          eleveId: m.eleveId,
                        })
                      }
                    >
                      Retirer
                    </button>
                  </li>
                ))}
                {!membres.length && (
                  <li className="text-slate-500">Aucun membre — ajoutez une classe ou un élève.</li>
                )}
              </ul>
            </>
          )}
        </section>
      </div>
    </ModulePageShell>
  );
}
