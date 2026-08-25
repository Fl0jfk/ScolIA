"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

type Periode = { id: string; code: string; libelle: string; statut: string };
type Groupe = { id: string; code: string; libelle: string; memberCount: number };
type PreviewEleve = {
  eleveId: string;
  nom: string;
  prenom: string;
  nbMatieres: number;
  nbCompetences?: number;
};

export default function NotesBulletinsClient() {
  const [periodes, setPeriodes] = useState<Periode[]>([]);
  const [groupes, setGroupes] = useState<Groupe[]>([]);
  const [scopeMode, setScopeMode] = useState<"classe" | "groupe">("classe");
  const [classe, setClasse] = useState("");
  const [groupeId, setGroupeId] = useState("");
  const [periodeId, setPeriodeId] = useState("");
  const [eleves, setEleves] = useState<PreviewEleve[]>([]);
  const [zipUrl, setZipUrl] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void (async () => {
      try {
        const [configRes, groupesRes] = await Promise.all([
          fetch("/api/notes/config", { cache: "no-store" }),
          fetch("/api/notes/competences", { cache: "no-store" }),
        ]);
        const configData = await configRes.json();
        const groupesData = groupesRes.ok ? await groupesRes.json() : { groupes: [] };
        if (!configRes.ok) throw new Error(configData?.error || "Chargement impossible");
        setPeriodes(configData.periodes || []);
        setGroupes(groupesData.groupes || []);
      } catch (e: unknown) {
        setError(e instanceof Error ? e.message : "Erreur");
      }
    })();
  }, []);

  const scopeReady = scopeMode === "groupe" ? Boolean(groupeId) : Boolean(classe.trim());

  const loadPreview = useCallback(async () => {
    if (!scopeReady || !periodeId) return;
    setBusy(true);
    setError(null);
    try {
      const body: Record<string, string> = { periodeId };
      if (scopeMode === "groupe") body.groupeId = groupeId;
      else body.classe = classe.trim();
      const res = await fetch("/api/notes/bulletins", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Prévisualisation impossible");
      setGroupes(data.groupes || []);
      setEleves(data.eleves || []);
      setZipUrl(data.zipUrl || null);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
      setEleves([]);
      setZipUrl(null);
    } finally {
      setBusy(false);
    }
  }, [scopeMode, classe, groupeId, periodeId, scopeReady]);

  useEffect(() => {
    if (scopeReady && periodeId) void loadPreview();
    else {
      setEleves([]);
      setZipUrl(null);
    }
  }, [scopeReady, periodeId, loadPreview]);

  const withContent = eleves.filter((e) => e.nbMatieres > 0 || (e.nbCompetences ?? 0) > 0);

  return (
    <ModulePageShell maxWidthClass="max-w-5xl">
      <ModulePageHeader
        eyebrow="Administratif · Notes Phase 3"
        title="Éditions bulletins PDF"
        description="Génération par classe ou groupe pédagogique — moyennes, MG et compétences LSU si saisies."
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
            <Link href="/notes/espace" className="text-sm font-bold text-indigo-600 hover:underline">
              ← Configuration
            </Link>
          </div>
        }
      />

      <div className="flex flex-wrap gap-3 mb-6 items-end">
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
              {p.libelle} ({p.statut})
            </option>
          ))}
        </select>
        {zipUrl && withContent.length > 0 ? (
          <a
            href={zipUrl}
            className="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-bold text-white hover:bg-indigo-700"
          >
            Télécharger ZIP ({withContent.length})
          </a>
        ) : null}
      </div>

      {error && <p className="mb-3 text-sm text-red-600">{error}</p>}
      {busy && <p className="mb-3 text-sm text-slate-500">Chargement…</p>}

      <section className="rounded-2xl border border-slate-200 bg-white p-5 overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-slate-500">
              <th className="py-2">Élève</th>
              <th>Matières</th>
              <th>Compétences</th>
              <th className="text-right">PDF</th>
            </tr>
          </thead>
          <tbody>
            {eleves.map((e) => (
              <tr key={e.eleveId} className="border-t border-slate-100">
                <td className="py-2 font-semibold">
                  {e.nom} {e.prenom}
                </td>
                <td>{e.nbMatieres || "—"}</td>
                <td>{e.nbCompetences || "—"}</td>
                <td className="text-right">
                  {(e.nbMatieres > 0 || (e.nbCompetences ?? 0) > 0) && periodeId ? (
                    <a
                      href={`/api/notes/bulletins/pdf?eleveId=${e.eleveId}&periodeId=${periodeId}`}
                      target="_blank"
                      rel="noreferrer"
                      className="text-indigo-600 font-bold hover:underline"
                    >
                      Ouvrir
                    </a>
                  ) : (
                    <span className="text-slate-400">—</span>
                  )}
                </td>
              </tr>
            ))}
            {!eleves.length && !busy && scopeReady && periodeId ? (
              <tr>
                <td colSpan={4} className="py-4 text-slate-500">
                  Aucun élève dans ce périmètre.
                </td>
              </tr>
            ) : null}
            {!scopeReady || !periodeId ? (
              <tr>
                <td colSpan={4} className="py-4 text-slate-500">
                  Renseignez {scopeMode === "groupe" ? "un groupe" : "une classe"} et une période pour
                  prévisualiser les bulletins.
                </td>
              </tr>
            ) : null}
          </tbody>
        </table>
      </section>
    </ModulePageShell>
  );
}
