"use client";

import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { dash } from "@/app/lib/dashboard-brand";

type Template = {
  key: string;
  label: string;
  calendrierMode: string;
  description: string;
  etapesCount: number;
};

type Campagne = {
  id: string;
  label: string;
  anneeLabel: string;
  calendrierMode: string;
  statut: string;
  templateKey: string | null;
  createdAt: string;
};

export default function FichesDialogueHubPage() {
  const [campagnes, setCampagnes] = useState<Campagne[]>([]);
  const [templates, setTemplates] = useState<Template[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [form, setForm] = useState({
    templateKey: "college_trimestriel",
    label: "",
    anneeLabel: "2025-2026",
    classesCibles: "",
    delaiFamilleJours: 7,
    appelEnabled: true,
    appelDateLimite: "",
    appelProcedure: "",
  });

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/fiches-dialogue/campagnes", { cache: "no-store" });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Erreur de chargement");
      setCampagnes(json.campagnes || []);
      setTemplates(json.templates || []);
      if (json.templates?.[0]?.key) {
        setForm((f) => ({ ...f, templateKey: f.templateKey || json.templates[0].key }));
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  async function createCampagne() {
    setCreating(true);
    setError(null);
    try {
      const res = await fetch("/api/fiches-dialogue/campagnes", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          templateKey: form.templateKey,
          label: form.label.trim() || `Fiches de dialogue ${form.anneeLabel}`,
          anneeLabel: form.anneeLabel.trim(),
          classesCibles: form.classesCibles
            .split(",")
            .map((s) => s.trim())
            .filter(Boolean),
          delaiFamilleJours: form.delaiFamilleJours,
          appelConfig: {
            enabled: form.appelEnabled,
            dateLimite: form.appelDateLimite || undefined,
            procedureHtml: form.appelProcedure || undefined,
            documentsLabels: [
              "Formulaire d’appel",
              "Décision du conseil de classe (fiche de dialogue)",
            ],
          },
        }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Création impossible");
      window.location.href = `/fiches-dialogue/${json.campagne.id}`;
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
      setCreating(false);
    }
  }

  if (loading) {
    return (
      <ModulePageShell maxWidthClass="max-w-[1100px]">
        <p className={`text-center text-sm ${dash.textMid}`}>Chargement…</p>
      </ModulePageShell>
    );
  }

  return (
    <ModulePageShell maxWidthClass="max-w-[1100px]">
      <ModulePageHeader
        title="Fiches de dialogue"
        description="Orientation année suivante — campagnes configurables (trimestre / semestre), vœux familles, conseils, acceptation et appel."
      />

      {error && (
        <p className="rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800">
          {error}
        </p>
      )}

      <ModuleCard bodyClassName="space-y-4 p-5">
        <h2 className={`text-lg font-semibold ${dash.ink}`}>Nouvelle campagne</h2>
        <div className="grid gap-3 sm:grid-cols-2">
          <label className="block text-sm">
            <span className={dash.textMid}>Modèle</span>
            <select
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              value={form.templateKey}
              onChange={(e) => setForm({ ...form, templateKey: e.target.value })}
            >
              {templates.map((t) => (
                <option key={t.key} value={t.key}>
                  {t.label} ({t.etapesCount} étapes)
                </option>
              ))}
            </select>
          </label>
          <label className="block text-sm">
            <span className={dash.textMid}>Année scolaire</span>
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              value={form.anneeLabel}
              onChange={(e) => setForm({ ...form, anneeLabel: e.target.value })}
            />
          </label>
          <label className="block text-sm sm:col-span-2">
            <span className={dash.textMid}>Libellé</span>
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              placeholder="Ex. Orientation collège 2025-2026"
              value={form.label}
              onChange={(e) => setForm({ ...form, label: e.target.value })}
            />
          </label>
          <label className="block text-sm sm:col-span-2">
            <span className={dash.textMid}>
              Classes cibles (séparées par des virgules — vide = toutes)
            </span>
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              placeholder="5e, 4e, 3e…"
              value={form.classesCibles}
              onChange={(e) => setForm({ ...form, classesCibles: e.target.value })}
            />
          </label>
          <label className="block text-sm">
            <span className={dash.textMid}>Délai famille (jours)</span>
            <input
              type="number"
              min={1}
              max={60}
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              value={form.delaiFamilleJours}
              onChange={(e) =>
                setForm({ ...form, delaiFamilleJours: Number(e.target.value) || 7 })
              }
            />
          </label>
          <label className="flex items-end gap-2 pb-2 text-sm">
            <input
              type="checkbox"
              checked={form.appelEnabled}
              onChange={(e) => setForm({ ...form, appelEnabled: e.target.checked })}
            />
            <span>Activer la procédure d’appel (si refus famille)</span>
          </label>
          <label className="block text-sm">
            <span className={dash.textMid}>Date limite d’appel</span>
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              placeholder="Ex. 15 juin 2026"
              value={form.appelDateLimite}
              onChange={(e) => setForm({ ...form, appelDateLimite: e.target.value })}
            />
          </label>
          <label className="block text-sm sm:col-span-2">
            <span className={dash.textMid}>Texte procédure d’appel</span>
            <textarea
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              rows={3}
              placeholder="Expliquez comment constituer le dossier d’appel…"
              value={form.appelProcedure}
              onChange={(e) => setForm({ ...form, appelProcedure: e.target.value })}
            />
          </label>
        </div>
        {templates.find((t) => t.key === form.templateKey)?.description && (
          <p className={`text-sm ${dash.textMid}`}>
            {templates.find((t) => t.key === form.templateKey)?.description}
          </p>
        )}
        <ModuleButton disabled={creating} onClick={() => void createCampagne()}>
          {creating ? "Création…" : "Créer la campagne"}
        </ModuleButton>
      </ModuleCard>

      <section className="space-y-3">
        <h2 className={`text-lg font-semibold ${dash.ink}`}>Campagnes</h2>
        {campagnes.length === 0 ? (
          <p className={`text-sm ${dash.textMid}`}>Aucune campagne pour l’instant.</p>
        ) : (
          <div className="grid gap-3">
            {campagnes.map((c) => (
              <Link key={c.id} href={`/fiches-dialogue/${c.id}`}>
                <ModuleCard
                  bodyClassName={`p-4 transition hover:-translate-y-0.5 ${dash.hoverBorder}`}
                  className="block"
                >
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <div>
                      <p className={`font-semibold ${dash.ink}`}>{c.label}</p>
                      <p className={`text-sm ${dash.textMid}`}>
                        {c.anneeLabel} · {c.calendrierMode} · {c.statut}
                      </p>
                    </div>
                    <span className="text-sm text-emerald-700">Ouvrir →</span>
                  </div>
                </ModuleCard>
              </Link>
            ))}
          </div>
        )}
      </section>
    </ModulePageShell>
  );
}
