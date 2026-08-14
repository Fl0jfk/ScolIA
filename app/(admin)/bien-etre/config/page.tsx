"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import type { BienEtreConfig } from "@/app/lib/bien-etre-types";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { dash } from "@/app/lib/dashboard-brand";

export default function BienEtreConfigPage() {
  const [config, setConfig] = useState<BienEtreConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/bien-etre/config", { cache: "no-store" });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      setConfig(j.config);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const save = async () => {
    if (!config) return;
    setSaving(true);
    setMsg(null);
    setError(null);
    try {
      const res = await fetch("/api/bien-etre/config", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(config),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      setConfig(j.config);
      setMsg("Configuration enregistrée.");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  return (
    <ModulePageShell maxWidthClass="max-w-2xl">
      <ModulePageHeader
        eyebrow="Élèves"
        title="Bot bien-être"
        description="Configuration — écoute élèves et signalements."
        actions={
          <Link href="/bien-etre/referent" className={`text-sm font-semibold underline ${dash.textPrimary}`}>
            Voir les signalements →
          </Link>
        }
      />

      {loading ? <p className={`text-sm ${dash.textMid}`}>Chargement…</p> : null}
      {error ? <p className="mb-4 text-sm text-rose-700">{error}</p> : null}
      {msg ? <p className={`mb-4 text-sm ${dash.textPrimary}`}>{msg}</p> : null}

      {config ? (
        <ModuleCard bodyClassName="space-y-5 p-6">
          <label className="flex cursor-pointer items-center gap-3">
            <input
              type="checkbox"
              checked={config.enabled}
              onChange={(e) => setConfig({ ...config, enabled: e.target.checked })}
              className="h-4 w-4 rounded border-[color:var(--dash-border)] text-[var(--dash-primary)]"
            />
            <span className={`font-semibold ${dash.ink}`}>Activer le bot bien-être pour les élèves</span>
          </label>

          <label className={`block text-sm font-semibold ${dash.ink}`}>
            E-mail du psychologue (destinataire des signalements)
            <input
              type="email"
              value={config.psychologistEmail}
              onChange={(e) => setConfig({ ...config, psychologistEmail: e.target.value })}
              className={`mt-1 ${dash.field}`}
              placeholder="psychologue@etablissement.fr"
            />
          </label>

          <label className={`block text-sm font-semibold ${dash.ink}`}>
            E-mail expéditeur des notifications (optionnel — compte SMTP dédié recommandé)
            <input
              type="email"
              value={config.notificationFromEmail || ""}
              onChange={(e) => setConfig({ ...config, notificationFromEmail: e.target.value })}
              className={`mt-1 ${dash.field}`}
              placeholder="notifications-bienetre@…"
            />
          </label>

          <label className={`block text-sm font-semibold ${dash.ink}`}>
            Rétention des signalements (jours)
            <input
              type="number"
              min={7}
              max={365}
              value={config.retentionDays}
              onChange={(e) => setConfig({ ...config, retentionDays: Number(e.target.value) || 90 })}
              className={`mt-1 ${dash.field}`}
            />
          </label>

          <label className={`block text-sm font-semibold ${dash.ink}`}>
            Message d&apos;accueil
            <textarea
              value={config.welcomeMessage || ""}
              onChange={(e) => setConfig({ ...config, welcomeMessage: e.target.value })}
              rows={4}
              className={`mt-1 ${dash.field}`}
            />
          </label>

          <p className={`text-xs ${dash.textMid}`}>
            Les comptes élèves utilisent la bulle <strong>💜 bien-être</strong> sur le tableau de bord (pas Nico).
            Référent : <code className={`rounded px-1 ${dash.bgSoft}`}>/bien-etre/referent</code>
          </p>

          <ModuleButton onClick={save} disabled={saving} className="px-6 py-3">
            {saving ? "Enregistrement…" : "Enregistrer"}
          </ModuleButton>
        </ModuleCard>
      ) : null}
    </ModulePageShell>
  );
}
