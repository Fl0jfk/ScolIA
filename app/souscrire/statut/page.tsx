"use client";

import { Suspense, useEffect, useState } from "react";
import Link from "@/app/components/ui/AppLink";
import { useSearchParams } from "next/navigation";
import MarketingShell from "@/app/components/landing/MarketingShell";
import { SCOLA_GRADIENT_TEXT } from "@/app/lib/marketing-theme";
import { SIGNUP_STATUS_LABELS, type TenantSignupStatus } from "@/app/lib/platform-signup-types";

type PublicStatus = {
  id: string;
  status: TenantSignupStatus;
  legalName: string;
  rne: string;
  createdAt: string;
  updatedAt: string;
  rejectedReason?: string;
  canPay: boolean;
  provisionedTenantSlug?: string;
};

function StatutInner() {
  const searchParams = useSearchParams();
  const token = searchParams.get("token") || "";
  const paidParam = searchParams.get("paid");
  const [data, setData] = useState<PublicStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!token) {
      setError("Lien invalide.");
      return;
    }
    void (async () => {
      try {
        const res = await fetch(`/api/platform/signup-requests/status?token=${encodeURIComponent(token)}`);
        const j = await res.json();
        if (!res.ok) throw new Error(j.error || "Erreur");
        setData(j.request);
      } catch (e) {
        setError(e instanceof Error ? e.message : "Erreur");
      }
    })();
  }, [token]);

  return (
    <main className="mx-auto max-w-xl px-6 py-10">
      <h1 className="text-2xl font-black text-[#14231A]">
        Suivi de <span className={SCOLA_GRADIENT_TEXT}>votre dossier</span>
      </h1>
      {error && (
        <p className="mt-4 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">{error}</p>
      )}
      {data && (
        <div className="mt-6 space-y-4 rounded-2xl border border-stone-200 bg-white/90 p-6 shadow-lg">
          {paidParam === "0" && data.canPay && (
            <p className="rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
              Le paiement n&apos;a pas été finalisé. Vous pouvez réessayer ci-dessous — votre dossier reste
              en attente.
            </p>
          )}
          <p className="text-sm text-stone-600">
            <strong>{data.legalName}</strong> — RNE {data.rne}
          </p>
          <div className="rounded-xl bg-amber-50 border border-amber-200 px-4 py-3">
            <p className="text-xs font-bold uppercase tracking-widest text-amber-800">Statut</p>
            <p className="mt-1 font-semibold text-amber-950">{SIGNUP_STATUS_LABELS[data.status]}</p>
          </div>
          {data.status === "pending_microsoft" && (
            <p className="text-sm text-stone-600 leading-relaxed">
              Votre dossier est <strong>en cours d&apos;étude</strong> pour vérifier l&apos;éligibilité
              Microsoft Education. Nous vous recontacterons par e-mail.
            </p>
          )}
          {data.rejectedReason && (
            <p className="text-sm text-red-700">
              <strong>Motif :</strong> {data.rejectedReason}
            </p>
          )}
          {data.canPay && (
            <Link
              href={`/souscrire/paiement?token=${encodeURIComponent(token)}`}
              className="inline-block rounded-xl bg-[#2F6B4A] px-5 py-2.5 text-sm font-bold text-white"
            >
              Choisir mon abonnement
            </Link>
          )}
          {data.status === "active" && data.provisionedTenantSlug && (
            <p className="text-sm text-emerald-800">
              Votre espace est prêt. Consultez votre e-mail pour le lien de connexion (
              <span className="font-mono">{data.provisionedTenantSlug}.scola.fr</span>).
            </p>
          )}
        </div>
      )}
    </main>
  );
}

export default function SouscrireStatutPage() {
  return (
    <MarketingShell>
      <Suspense fallback={<p className="p-10 text-center text-stone-500">Chargement…</p>}>
        <StatutInner />
      </Suspense>
    </MarketingShell>
  );
}
