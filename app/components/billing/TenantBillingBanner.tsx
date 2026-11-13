"use client";

import { useEffect, useState } from "react";
import Link from "@/app/components/ui/AppLink";
import { MARKETING } from "@/app/lib/marketing-site";

type BillingInfo = {
  status: string;
  graceEndsAt?: string;
  lastFailureAt?: string;
  graceDays: number;
};

export default function TenantBillingBanner() {
  const [billing, setBilling] = useState<BillingInfo | null>(null);

  useEffect(() => {
    void (async () => {
      try {
        const res = await fetch("/api/app/context", { cache: "no-store" });
        const j = await res.json();
        if (res.ok && j.billing?.status === "past_due") {
          setBilling(j.billing);
        }
      } catch {
        /* ignore */
      }
    })();
  }, []);

  if (!billing) return null;

  const graceLabel = billing.graceEndsAt
    ? new Date(billing.graceEndsAt).toLocaleDateString("fr-FR")
    : null;

  return (
    <div className="mx-4 mt-3 rounded-xl border border-amber-300 bg-amber-50 px-4 py-3 text-sm text-amber-950 shadow-sm">
      <p className="font-semibold">Paiement en retard</p>
      <p className="mt-1 text-amber-900/90">
        Un prélèvement n&apos;a pas abouti. Votre accès reste actif
        {graceLabel ? ` jusqu'au ${graceLabel}` : ` pendant ${billing.graceDays} jours`}, le temps de
        régulariser. Sans action, l&apos;accès pourra être suspendu — vos données seront conservées.
      </p>
      <p className="mt-2 text-xs text-amber-800">
        Contact :{" "}
        <a href={`mailto:${MARKETING.contactEmail}`} className="font-semibold underline">
          {MARKETING.contactEmail}
        </a>
        {" · "}
        <Link href="/assistance" className="font-semibold underline">
          Assistance
        </Link>
      </p>
    </div>
  );
}
