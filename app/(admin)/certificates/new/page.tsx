"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { currentCertificateSchoolYear } from "@/app/lib/certificates-types";

export default function NewCertificateProgramPage() {
  const router = useRouter();
  const [title, setTitle] = useState("");
  const [schoolYear, setSchoolYear] = useState(currentCertificateSchoolYear());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function create() {
    const t = title.trim();
    if (!t) {
      setError("Titre requis.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/certificates/programs", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ title: t, schoolYear }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      router.push(`/certificates/${data.program.id}`);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  return (
    <ModulePageShell maxWidthClass="max-w-lg" className="space-y-6">
      <ModulePageHeader eyebrow="Élèves" title="Nouveau parcours" />
      <ModuleCard className="space-y-4 p-5">
        <label className="block space-y-1">
          <span className="text-sm font-bold text-slate-700">Titre du certificat / parcours</span>
          <input
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="Ex. Certificat d'excellence sportive"
            className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
          />
        </label>
        <label className="block space-y-1">
          <span className="text-sm font-bold text-slate-700">Année scolaire</span>
          <input
            value={schoolYear}
            onChange={(e) => setSchoolYear(e.target.value)}
            className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
          />
        </label>
        {error && <p className="text-sm text-red-600">{error}</p>}
        <ModuleButton className="w-full py-3" disabled={busy} onClick={() => void create()}>
          {busy ? "Création…" : "Créer le parcours"}
        </ModuleButton>
      </ModuleCard>
    </ModulePageShell>
  );
}
