"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

type Domaine = { id: string; code: string; libelle: string; cycle: string };
type Item = { id: string; domaineId: string; code: string; libelle: string };
type Periode = { id: string; code: string; libelle: string; statut: string };
type Eleve = { id: string; nom: string; prenom: string };
type Valeur = { itemId: string; eleveId: string; niveau: string | null };
type Niveau = { code: string; label: string };
type Groupe = { id: string; code: string; libelle: string; memberCount: number };

export default function NotesCompetencesClient() {
  const [domaines, setDomaines] = useState<Domaine[]>([]);
  const [items, setItems] = useState<Item[]>([]);
  const [periodes, setPeriodes] = useState<Periode[]>([]);
  const [groupes, setGroupes] = useState<Groupe[]>([]);
  const [eleves, setEleves] = useState<Eleve[]>([]);
  const [valeurs, setValeurs] = useState<Valeur[]>([]);
  const [niveaux, setNiveaux] = useState<Niveau[]>([]);
  const [scopeMode, setScopeMode] = useState<"classe" | "groupe">("classe");
  const [classe, setClasse] = useState("");
  const [groupeId, setGroupeId] = useState("");
  const [periodeId, setPeriodeId] = useState("");
  const [domaineId, setDomaineId] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const valeurMap = useMemo(() => {
    const m = new Map<string, string | null>();
    for (const v of valeurs) m.set(`${v.itemId}:${v.eleveId}`, v.niveau);
    return m;
  }, [valeurs]);

  const load = useCallback(async () => {
    setError(null);
    const qs = new URLSearchParams();
    if (scopeMode === "groupe" && groupeId) qs.set("groupeId", groupeId);
    else if (classe.trim()) qs.set("classe", classe.trim());
    if (periodeId) qs.set("periodeId", periodeId);
    if (domaineId) qs.set("domaineId", domaineId);
    const res = await fetch(`/api/notes/competences?${qs}`, { cache: "no-store" });
    const data = await res.json();
    if (!res.ok) throw new Error(data?.error || "Chargement impossible");
    setDomaines(data.domaines || []);
    setItems(data.items || []);
    setPeriodes(data.periodes || []);
    setGroupes(data.groupes || []);
    setEleves(data.eleves || []);
    setValeurs(data.valeurs || []);
    setNiveaux(data.niveaux || []);
  }, [scopeMode, classe, groupeId, periodeId, domaineId]);

  useEffect(() => {
    void load().catch((e: unknown) => {
      setError(e instanceof Error ? e.message : "Erreur");
    });
  }, [load]);

  const post = async (body: Record<string, unknown>) => {
    setBusy(true);
    setMessage(null);
    setError(null);
    try {
      const res = await fetch("/api/notes/competences", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Échec");
      await load();
      return data;
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
      return null;
    } finally {
      setBusy(false);
    }
  };

  const scopeReady = scopeMode === "groupe" ? Boolean(groupeId) : Boolean(classe.trim());

  const exportLsu = async () => {
    if (!scopeReady || !periodeId) return;
    setBusy(true);
    setError(null);
    try {
      const qs = new URLSearchParams({ export: "lsu", periodeId });
      if (scopeMode === "groupe") qs.set("groupeId", groupeId);
      else qs.set("classe", classe.trim());
      const res = await fetch(`/api/notes/competences?${qs}`);
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Export impossible");
      const blob = new Blob([JSON.stringify(data.rows, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `lsu-${scopeMode === "groupe" ? groupeId.slice(0, 8) : classe.trim()}-${periodeId.slice(0, 8)}.json`;
      a.click();
      URL.revokeObjectURL(url);
      setMessage(`${data.count ?? 0} évaluation(s) exportée(s) (JSON LSU V1).`);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  return (
    <ModulePageShell maxWidthClass="max-w-6xl">
      <ModulePageHeader
        eyebrow="Administratif · Notes · Compétences LSU"
        title="Compétences collège"
        description="Grille de maîtrise par domaine (Option B) — complète les notes chiffrées pour bulletins collège et export LSU."
        actions={
          <Link href="/notes/espace" className="text-sm font-bold text-indigo-600 hover:underline">
            ← Configuration notes
          </Link>
        }
      />

      <div className="flex flex-wrap gap-3 mb-4 items-end">
        <div className="flex rounded-xl border border-slate-200 overflow-hidden text-sm font-bold">
          <button
            type="button"
            onClick={() => {
              setScopeMode("classe");
              setGroupeId("");
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
              {p.libelle}
            </option>
          ))}
        </select>
        <select
          className="border rounded-xl px-3 py-2 text-sm min-w-[12rem]"
          value={domaineId}
          onChange={(e) => setDomaineId(e.target.value)}
        >
          <option value="">Domaine</option>
          {domaines.map((d) => (
            <option key={d.id} value={d.id}>
              {d.code} — {d.libelle}
            </option>
          ))}
        </select>
        <button
          type="button"
          disabled={busy}
          onClick={() => void post({ action: "seedDefaults" })}
          className="rounded-xl border border-slate-200 px-3 py-2 text-sm font-bold"
        >
          Initialiser domaines collège
        </button>
        {scopeReady && periodeId ? (
          <button
            type="button"
            disabled={busy}
            onClick={() => void exportLsu()}
            className="rounded-xl bg-indigo-600 text-white px-3 py-2 text-sm font-bold disabled:opacity-50"
          >
            Export LSU (JSON)
          </button>
        ) : null}
      </div>

      {message && <p className="mb-3 text-sm text-emerald-700 font-semibold">{message}</p>}
      {error && <p className="mb-3 text-sm text-red-600">{error}</p>}

      {!domaineId || !periodeId || !scopeReady ? (
        <p className="text-sm text-slate-500">
          Renseignez {scopeMode === "groupe" ? "un groupe" : "une classe"}, période et domaine pour
          saisir les niveaux de maîtrise (1–4).
        </p>
      ) : (
        <section className="rounded-2xl border border-slate-200 bg-white p-4 overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-slate-500">
                <th className="py-2 pr-3 sticky left-0 bg-white">Élève</th>
                {items.map((item) => (
                  <th key={item.id} className="py-2 px-2 min-w-[8rem]">
                    <span className="font-mono text-xs block">{item.code}</span>
                    <span className="font-normal text-xs">{item.libelle}</span>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {eleves.map((e) => (
                <tr key={e.id} className="border-t border-slate-100">
                  <td className="py-2 pr-3 sticky left-0 bg-white font-semibold whitespace-nowrap">
                    {e.nom} {e.prenom}
                  </td>
                  {items.map((item) => {
                    const key = `${item.id}:${e.id}`;
                    const current = valeurMap.get(key) || "";
                    return (
                      <td key={item.id} className="py-1 px-1">
                        <select
                          className="border rounded-lg px-1 py-1 text-xs w-full"
                          value={current}
                          disabled={busy}
                          onChange={(ev) => {
                            void post({
                              action: "upsertValeur",
                              itemId: item.id,
                              eleveId: e.id,
                              periodeId,
                              niveau: ev.target.value || null,
                            });
                          }}
                        >
                          <option value="">—</option>
                          {niveaux.map((n) => (
                            <option key={n.code} value={n.code}>
                              {n.code} · {n.label}
                            </option>
                          ))}
                        </select>
                      </td>
                    );
                  })}
                </tr>
              ))}
              {!eleves.length && (
                <tr>
                  <td colSpan={items.length + 1} className="py-4 text-slate-500">
                    Aucun élève dans ce périmètre.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </section>
      )}
    </ModulePageShell>
  );
}
