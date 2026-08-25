"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import type { PaieRhSpecSnapshot } from "@/app/lib/paie-rh-spec";

export default function RhPaieSpecPage() {
  const [spec, setSpec] = useState<PaieRhSpecSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void (async () => {
      try {
        const res = await fetch("/api/rh/paie-spec", { cache: "no-store" });
        const data = await res.json();
        if (!res.ok) throw new Error(data?.error || "Chargement impossible");
        setSpec(data as PaieRhSpecSnapshot);
      } catch (e: unknown) {
        setError(e instanceof Error ? e.message : "Erreur");
      }
    })();
  }, []);

  return (
    <ModulePageShell>
      <ModulePageHeader
        title="Paie RH — spec Phase 1c"
        description="Contrat produit figé. Implémentation après brief comptable (conventions OGEC limitées)."
      />
      {error ? <p className="text-sm text-red-600">{error}</p> : null}
      {!spec && !error ? <p className="text-sm text-slate-500">Chargement…</p> : null}
      {spec ? (
        <div className="space-y-6">
          <section className="rounded-2xl border border-amber-200 bg-amber-50 px-5 py-4 text-sm text-amber-950">
            <p className="font-black">Statut : {spec.status}</p>
            <p className="mt-1">Version {spec.version}</p>
            <p className="mt-2 text-xs">
              Bloquants : {spec.blockers.join(" · ")}
            </p>
          </section>
          <section className="rounded-2xl border border-slate-200 bg-white p-5">
            <h2 className="font-black text-slate-900">Dans le périmètre</h2>
            <ul className="mt-3 list-disc space-y-1 pl-5 text-sm text-slate-700">
              {spec.inScope.map((x) => (
                <li key={x}>{x}</li>
              ))}
            </ul>
          </section>
          <section className="rounded-2xl border border-slate-200 bg-white p-5">
            <h2 className="font-black text-slate-900">Hors périmètre</h2>
            <ul className="mt-3 list-disc space-y-1 pl-5 text-sm text-slate-700">
              {spec.outOfScope.map((x) => (
                <li key={x}>{x}</li>
              ))}
            </ul>
          </section>
          <section className="rounded-2xl border border-slate-200 bg-white p-5">
            <h2 className="font-black text-slate-900">Entités cibles</h2>
            <ul className="mt-3 list-disc space-y-1 pl-5 font-mono text-xs text-slate-700">
              {spec.entities.map((x) => (
                <li key={x}>{x}</li>
              ))}
            </ul>
          </section>
          <Link href="/rh/moi" className="text-sm font-bold text-indigo-600 hover:underline">
            ← Mon dossier RH
          </Link>
        </div>
      ) : null}
    </ModulePageShell>
  );
}
