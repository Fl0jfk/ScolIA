"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "@/app/components/ui/AppLink";
import { useParams } from "next/navigation";
import MarketingShell from "@/app/components/landing/MarketingShell";
import RequirePlatformMaster from "@/app/components/RequirePlatformMaster";
import {
  BILLING_GRACE_DAYS,
  BILLING_STATUS_LABELS,
  MICROSOFT_LICENSES_BILLING_LABELS,
  type TenantBillingState,
} from "@/app/lib/tenant-billing-types";
import {
  computePricingWithA3Extras,
  EXTRA_A3_PRICE_MONTHLY_EUR,
  INCLUDED_A3_LICENSES,
  formatEur,
} from "@/app/lib/pricing";

export default function PlateformeTenantBillingPage() {
  const params = useParams();
  const slug = String(params.slug || "");
  const [billing, setBilling] = useState<TenantBillingState | null>(null);
  const [label, setLabel] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reason, setReason] = useState("");
  const [extraA3Count, setExtraA3Count] = useState(0);

  const reload = useCallback(async () => {
    const res = await fetch(`/api/platform/setup/tenants/${slug}/billing`, { cache: "no-store" });
    const j = await res.json();
    if (res.ok) {
      setBilling(j.billing);
      setLabel(j.label);
      setExtraA3Count(j.billing.extraA3Count || 0);
    } else {
      setError(j.error || "Chargement impossible");
    }
  }, [slug]);

  useEffect(() => {
    reload();
  }, [reload]);

  const action = async (actionName: string) => {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/platform/setup/tenants/${slug}/billing`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: actionName,
          reason: reason || undefined,
          extraA3Count,
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Action impossible");
      setBilling(j.billing);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const runDunning = async () => {
    setBusy(true);
    try {
      await fetch("/api/billing/dunning/process", { method: "POST" });
      await reload();
    } finally {
      setBusy(false);
    }
  };

  return (
    <RequirePlatformMaster redirectTo="/">
      <MarketingShell>
        <main className="mx-auto max-w-2xl px-6 py-10 space-y-6">
          <p className="text-sm text-stone-500">
            <Link href="/plateforme" className="text-[#2F6B4A] hover:underline">
              ← Plateforme
            </Link>
          </p>
          <h1 className="text-2xl font-black text-[#14231A]">
            Facturation — {label || slug}
          </h1>

          {error && (
            <p className="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">{error}</p>
          )}

          {billing && (
            <div className="rounded-2xl border border-stone-200 bg-white/90 p-6 space-y-4 text-sm">
              {(() => {
                const pricing = computePricingWithA3Extras(
                  billing.estimatedStudentCount || 0,
                  billing.billingMode || "monthly",
                  extraA3Count,
                );
                return (
                  <div className="rounded-xl border border-stone-200 bg-stone-50 p-3 text-xs space-y-1">
                    <p>
                      Base{" "}
                      {billing.billingMode === "annual_upfront"
                        ? formatEur(pricing.annualTotal)
                        : `${formatEur(pricing.monthlyTotal)} / mois`}
                    </p>
                    <p>
                      Supplément A3 ({extraA3Count} ×{" "}
                      {formatEur(EXTRA_A3_PRICE_MONTHLY_EUR, { decimals: 2 })}/mois) :{" "}
                      {billing.billingMode === "annual_upfront"
                        ? formatEur(pricing.extraA3AnnualTotal)
                        : `${formatEur(pricing.extraA3MonthlyTotal)} / mois`}
                    </p>
                    <p className="font-semibold">
                      Total envoyé à Easytransac :{" "}
                      {billing.billingMode === "annual_upfront"
                        ? formatEur(pricing.totalAnnualWithExtras)
                        : `${formatEur(pricing.totalMonthlyWithExtras)} / mois`}
                    </p>
                  </div>
                );
              })()}
              <div className="grid sm:grid-cols-2 gap-3">
                <div>
                  <p className="text-xs font-bold uppercase text-stone-500">Statut abonnement</p>
                  <p className="font-semibold text-stone-900">{BILLING_STATUS_LABELS[billing.status]}</p>
                </div>
                <div>
                  <p className="text-xs font-bold uppercase text-stone-500">Mode</p>
                  <p>{billing.billingMode === "annual_upfront" ? "Annuel" : "Mensuel"}</p>
                </div>
                <div>
                  <p className="text-xs font-bold uppercase text-stone-500">Dernier paiement</p>
                  <p>
                    {billing.lastPaymentAt
                      ? new Date(billing.lastPaymentAt).toLocaleString("fr-FR")
                      : "—"}
                  </p>
                </div>
                <div>
                  <p className="text-xs font-bold uppercase text-stone-500">Échecs</p>
                  <p>{billing.failureCount || 0}</p>
                </div>
                {billing.graceEndsAt && (
                  <div>
                    <p className="text-xs font-bold uppercase text-stone-500">Fin de grâce</p>
                    <p>{new Date(billing.graceEndsAt).toLocaleDateString("fr-FR")}</p>
                  </div>
                )}
                {billing.microsoftLicenses && (
                  <div>
                    <p className="text-xs font-bold uppercase text-stone-500">Licences Microsoft</p>
                    <p>{MICROSOFT_LICENSES_BILLING_LABELS[billing.microsoftLicenses.status]}</p>
                  </div>
                )}
                <div>
                  <p className="text-xs font-bold uppercase text-stone-500">A3 incluses / extra</p>
                  <p>
                    {billing.includedA3Count ?? INCLUDED_A3_LICENSES} incluses ·{" "}
                    {billing.extraA3Count || 0} supplémentaires
                  </p>
                </div>
              </div>

              <p className="text-xs text-stone-500">
                Période de grâce par défaut : {BILLING_GRACE_DAYS} jours après un échec avant suspension
                automatique (via relances).
              </p>

              <label className="block">
                <span className="text-stone-600">Motif (optionnel)</span>
                <input
                  className="mt-1 w-full rounded-xl border border-stone-200 px-3 py-2"
                  value={reason}
                  onChange={(e) => setReason(e.target.value)}
                  placeholder="Ex. impayé 3 mois"
                />
              </label>
              <label className="block">
                <span className="text-stone-600">
                  Licences A3 supplémentaires (au-delà des {billing.includedA3Count ?? INCLUDED_A3_LICENSES} incluses)
                </span>
                <div className="mt-1 flex gap-2">
                  <input
                    type="number"
                    min={0}
                    className="w-full rounded-xl border border-stone-200 px-3 py-2"
                    value={extraA3Count}
                    onChange={(e) => setExtraA3Count(Math.max(0, Number(e.target.value) || 0))}
                  />
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => action("set_extra_a3")}
                    className="rounded-lg border border-stone-300 px-3 py-2 text-xs font-semibold"
                  >
                    Enregistrer
                  </button>
                </div>
              </label>

              <div className="flex flex-wrap gap-2">
                {billing.status !== "suspended" && billing.status !== "cancelled" && (
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => action("suspend")}
                    className="rounded-lg bg-red-700 px-4 py-2 text-white text-sm font-semibold disabled:opacity-50"
                  >
                    Suspendre l&apos;accès
                  </button>
                )}
                {(billing.status === "suspended" || billing.status === "past_due") && (
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => action("reactivate")}
                    className="rounded-lg bg-[#2F6B4A] px-4 py-2 text-white text-sm font-semibold disabled:opacity-50"
                  >
                    Réactiver
                  </button>
                )}
                {billing.microsoftLicenses?.status === "active" && (
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => action("microsoft_suspend")}
                    className="rounded-lg border border-amber-400 px-4 py-2 text-amber-900 text-sm font-semibold"
                  >
                    Demander suspension licences MS
                  </button>
                )}
                {billing.microsoftLicenses?.status === "suspend_requested" && (
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => action("microsoft_revoke")}
                    className="rounded-lg border border-stone-400 px-4 py-2 text-sm font-semibold"
                  >
                    Marquer licences révoquées
                  </button>
                )}
                <button
                  type="button"
                  disabled={busy}
                  onClick={async () => {
                    setBusy(true);
                    setError(null);
                    try {
                      const res = await fetch(`/api/billing/easytransac/tenant-charge/${slug}`, {
                        method: "POST",
                      });
                      const j = await res.json();
                      if (!res.ok) throw new Error(j.error || "Erreur");
                      if (j.redirectUrl) window.open(j.redirectUrl, "_blank");
                    } catch (e) {
                      setError(e instanceof Error ? e.message : "Erreur");
                    } finally {
                      setBusy(false);
                    }
                  }}
                  className="rounded-lg border border-[#2F6B4A]/40 px-4 py-2 text-sm text-[#2F6B4A] font-semibold disabled:opacity-50"
                >
                  Déclencher prélèvement SDD
                </button>
                <button
                  type="button"
                  disabled={busy}
                  onClick={runDunning}
                  className="rounded-lg border border-stone-300 px-4 py-2 text-sm"
                >
                  Lancer relances / auto-suspension
                </button>
              </div>

              {billing.auditLog && billing.auditLog.length > 0 && (
                <div className="pt-4 border-t border-stone-100">
                  <p className="text-xs font-bold uppercase text-stone-500 mb-2">Historique</p>
                  <ul className="max-h-48 overflow-y-auto space-y-1 text-xs text-stone-600 font-mono">
                    {[...billing.auditLog].reverse().map((e, i) => (
                      <li key={`${e.at}-${i}`}>
                        {e.at.slice(0, 19)} — {e.action}
                        {e.detail ? ` (${e.detail})` : ""}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          )}
        </main>
      </MarketingShell>
    </RequirePlatformMaster>
  );
}
