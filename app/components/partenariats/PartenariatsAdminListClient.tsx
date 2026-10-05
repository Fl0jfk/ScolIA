"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import RequireModuleAccess from "@/app/components/RequireModuleAccess";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import {
  PARTENARIAT_CYCLE_LABELS,
  type PartenariatOffreRecord,
} from "@/app/lib/partenariats-types";

export default function PartenariatsAdminListClient() {
  const [offres, setOffres] = useState<PartenariatOffreRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [kind, setKind] = useState<"info" | "inscription">("info");
  const [creating, setCreating] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/partenariats/offres", { cache: "no-store" });
      const data = (await res.json()) as {
        offres?: PartenariatOffreRecord[];
        error?: string;
      };
      if (!res.ok) throw new Error(data.error || "Chargement impossible.");
      setOffres(data.offres || []);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function createOffre() {
    setCreating(true);
    setError(null);
    try {
      const res = await fetch("/api/partenariats/offres", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          title: title.trim() || "Nouveau partenariat",
          kind,
          enabled: false,
          cycles: ["college", "lycee"],
        }),
      });
      const data = (await res.json()) as {
        offre?: PartenariatOffreRecord;
        error?: string;
      };
      if (!res.ok) throw new Error(data.error || "Création impossible.");
      setTitle("");
      if (data.offre) {
        window.location.href = `/etablissement/partenariats/${data.offre.id}`;
        return;
      }
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setCreating(false);
    }
  }

  return (
    <RequireModuleAccess moduleId="partenariats">
      <ModulePageShell>
        <ModulePageHeader
          eyebrow="Établissement"
          title="Partenariats & offres"
          description="Catalogue public des partenaires et dispositifs (Dual Diploma, Voltaire, BIA…). Une fiche = une page familles + contacts clairs."
          actions={
            <a
              href="/partenariats"
              target="_blank"
              rel="noreferrer"
              className="rounded-xl border border-slate-200 px-3 py-2 text-xs font-bold text-slate-700 hover:bg-slate-50"
            >
              Voir la page publique
            </a>
          }
        />

        <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
          <h2 className="text-sm font-black text-slate-900">Nouvelle fiche</h2>
          <div className="flex flex-wrap items-end gap-3">
            <label className="flex flex-col gap-1 text-xs font-semibold text-slate-600">
              Titre
              <input
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                className="rounded-xl border border-slate-200 px-3 py-2 text-sm font-medium text-slate-900 min-w-[240px]"
                placeholder="Ex. Dual Diploma, FCR, Certificat Voltaire…"
              />
            </label>
            <label className="flex flex-col gap-1 text-xs font-semibold text-slate-600">
              Type
              <select
                value={kind}
                onChange={(e) =>
                  setKind(e.target.value === "inscription" ? "inscription" : "info")
                }
                className="rounded-xl border border-slate-200 px-3 py-2 text-sm font-medium text-slate-900"
              >
                <option value="info">Information / partenaire</option>
                <option value="inscription">Inscription en ligne</option>
              </select>
            </label>
            <ModuleButton type="button" onClick={() => void createOffre()} disabled={creating}>
              {creating ? "Création…" : "Créer"}
            </ModuleButton>
          </div>
        </section>

        {error ? (
          <p className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
            {error}
          </p>
        ) : null}

        <section className="space-y-3">
          <h2 className="text-sm font-black text-slate-900">Fiches</h2>
          {loading ? (
            <p className="text-sm text-slate-500">Chargement…</p>
          ) : offres.length === 0 ? (
            <p className="text-sm text-slate-500">Aucune fiche pour l’instant.</p>
          ) : (
            <ul className="grid gap-3 sm:grid-cols-2">
              {offres.map((o) => (
                <li
                  key={o.id}
                  className="rounded-2xl border border-slate-200 bg-white p-4 flex flex-col gap-2"
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0">
                      <p className="font-bold text-slate-900 truncate">{o.title}</p>
                      <p className="text-xs text-slate-500 mt-0.5">
                        /partenariats/{o.slug}
                      </p>
                    </div>
                    <span
                      className={`shrink-0 rounded-full px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide ${
                        o.enabled
                          ? "bg-emerald-50 text-emerald-700"
                          : "bg-slate-100 text-slate-500"
                      }`}
                    >
                      {o.enabled ? "Publié" : "Brouillon"}
                    </span>
                  </div>
                  <p className="text-xs text-slate-600 line-clamp-2">
                    {o.shortDescription || "—"}
                  </p>
                  <div className="flex flex-wrap gap-1.5 text-[11px] font-semibold text-slate-500">
                    <span className="rounded-md bg-slate-50 px-1.5 py-0.5">
                      {o.kind === "inscription" ? "Inscription" : "Info"}
                    </span>
                    {o.categoryLabel ? (
                      <span className="rounded-md bg-slate-50 px-1.5 py-0.5">
                        {o.categoryLabel}
                      </span>
                    ) : null}
                    {o.cycles.map((c) => (
                      <span key={c} className="rounded-md bg-slate-50 px-1.5 py-0.5">
                        {PARTENARIAT_CYCLE_LABELS[c]}
                      </span>
                    ))}
                    {o.kind === "inscription" ? (
                      <span className="rounded-md bg-sky-50 px-1.5 py-0.5 text-sky-700">
                        {o.inscriptionsCount || 0} inscrit
                        {(o.inscriptionsCount || 0) > 1 ? "s" : ""}
                      </span>
                    ) : null}
                  </div>
                  <Link
                    href={`/etablissement/partenariats/${o.id}`}
                    className="mt-1 inline-flex text-sm font-bold text-sky-700 hover:underline"
                  >
                    Configurer →
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </section>
      </ModulePageShell>
    </RequireModuleAccess>
  );
}
