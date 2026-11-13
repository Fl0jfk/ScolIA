"use client";

import { Suspense, useState } from "react";
import Link from "@/app/components/ui/AppLink";
import { useSearchParams } from "next/navigation";
import MarketingShell from "@/app/components/landing/MarketingShell";
import { SCOLA_GRADIENT_TEXT } from "@/app/lib/marketing-theme";
import {
  BILLING_OPTIONS,
  computePricingWithA3Extras,
  EXTRA_A3_PRICE_MONTHLY_EUR,
  INCLUDED_A3_LICENSES,
  formatEur,
  type BillingMode,
} from "@/app/lib/pricing";

function PaiementInner() {
  const searchParams = useSearchParams();
  const token = searchParams.get("token") || "";
  const [mode, setMode] = useState<BillingMode>("monthly");
  const [students, setStudents] = useState(200);
  const [extraA3Count, setExtraA3Count] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const pricing = computePricingWithA3Extras(students, mode, extraA3Count);

  const pay = async () => {
    if (!token) {
      setError("Lien invalide.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/billing/easytransac/checkout", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ token, mode, extraA3Count }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || j.message || "Erreur");
      if (j.redirectUrl) {
        window.location.href = j.redirectUrl;
        return;
      }
      throw new Error("URL de paiement manquante.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
      setBusy(false);
    }
  };

  return (
    <main className="mx-auto max-w-xl px-6 py-10">
      <h1 className="text-2xl font-black text-[#14231A]">
        Choix de l&apos;<span className={SCOLA_GRADIENT_TEXT}>abonnement</span>
      </h1>
      <p className="mt-2 text-sm text-stone-600">
        Paiement sécurisé via Easytransac. Votre établissement a été validé pour Microsoft Education.
      </p>

      <div className="mt-6 space-y-4 rounded-2xl border border-stone-200 bg-white/90 p-6 shadow-lg">
        <label className="block space-y-1">
          <span className="text-sm font-semibold text-stone-700">Effectif facturé (élèves)</span>
          <input
            type="number"
            min={1}
            value={students}
            onChange={(e) => setStudents(Number(e.target.value) || 0)}
            className="w-full rounded-xl border border-stone-200 px-4 py-2.5 text-sm"
          />
          <span className="text-xs text-stone-500">Reprend l&apos;effectif déclaré à l&apos;inscription.</span>
        </label>
        <label className="block space-y-1">
          <span className="text-sm font-semibold text-stone-700">
            Licences A3 supplémentaires (au-delà des {INCLUDED_A3_LICENSES} incluses)
          </span>
          <input
            type="number"
            min={0}
            value={extraA3Count}
            onChange={(e) => setExtraA3Count(Math.max(0, Number(e.target.value) || 0))}
            className="w-full rounded-xl border border-stone-200 px-4 py-2.5 text-sm"
          />
          <span className="text-xs text-stone-500">
            {formatEur(EXTRA_A3_PRICE_MONTHLY_EUR, { decimals: 2 })} / mois / licence
          </span>
        </label>

        <div className="grid gap-3">
          {BILLING_OPTIONS.map((opt) => (
            <button
              key={opt.id}
              type="button"
              onClick={() => setMode(opt.mode)}
              className={`rounded-xl border p-4 text-left transition ${
                mode === opt.mode
                  ? "border-[#2F6B4A] bg-emerald-50"
                  : "border-stone-200 hover:border-stone-300"
              }`}
            >
              <p className="font-bold text-[#14231A]">{opt.name}</p>
              <p className="text-sm text-stone-600">{opt.summary}</p>
              <p className="mt-2 font-mono text-sm text-[#2F6B4A]">
                {mode === opt.mode
                  ? formatEur(
                      opt.mode === "monthly"
                        ? pricing.totalMonthlyWithExtras
                        : pricing.totalAnnualWithExtras,
                    )
                  : opt.priceLine}
              </p>
            </button>
          ))}
        </div>
        {extraA3Count > 0 && (
          <p className="text-xs text-stone-600">
            Supplément A3 :{" "}
            {mode === "monthly"
              ? `${formatEur(pricing.extraA3MonthlyTotal)} / mois`
              : `${formatEur(pricing.extraA3AnnualTotal)} / an`}
          </p>
        )}

        {error && (
          <p className="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">{error}</p>
        )}

        <button
          type="button"
          disabled={busy}
          onClick={() => void pay()}
          className="w-full rounded-xl bg-[#2F6B4A] py-3 text-sm font-bold text-white disabled:opacity-40"
        >
          {busy ? "Redirection…" : "Payer avec Easytransac"}
        </button>

        <Link
          href={`/souscrire/statut?token=${encodeURIComponent(token)}`}
          className="block text-center text-sm text-stone-500 hover:underline"
        >
          Retour au suivi de dossier
        </Link>
      </div>
    </main>
  );
}

export default function SouscrirePaiementPage() {
  return (
    <MarketingShell>
      <Suspense fallback={<p className="p-10 text-center text-stone-500">Chargement…</p>}>
        <PaiementInner />
      </Suspense>
    </MarketingShell>
  );
}
