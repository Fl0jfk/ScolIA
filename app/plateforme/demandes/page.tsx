"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "@/app/components/ui/AppLink";
import MarketingShell from "@/app/components/landing/MarketingShell";
import RequirePlatformMaster from "@/app/components/RequirePlatformMaster";
import { SCOLA_GRADIENT_TEXT } from "@/app/lib/marketing-theme";
import { SIGNUP_STATUS_LABELS, type TenantSignupStatus } from "@/app/lib/platform-signup-types";

type Row = {
  id: string;
  status: TenantSignupStatus;
  legalName: string;
  rne: string;
  adminEmail: string;
  createdAt: string;
  updatedAt: string;
};

const FILTERS: Array<TenantSignupStatus | "all"> = [
  "all",
  "pending_microsoft",
  "microsoft_approved",
  "pending_payment",
  "payment_completed",
  "active",
  "rejected",
];

export default function PlateformeDemandesPage() {
  const [rows, setRows] = useState<Row[]>([]);
  const [filter, setFilter] = useState<(typeof FILTERS)[number]>("all");
  const [loading, setLoading] = useState(true);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch("/api/platform/signup-requests", { cache: "no-store" });
      const j = await res.json();
      if (res.ok) {
        setRows(
          (j.requests || []).map((r: Row & { establishment?: { legalName: string; rne: string }; adminContact?: { email: string } }) => ({
            id: r.id,
            status: r.status,
            legalName: r.establishment?.legalName || r.legalName,
            rne: r.establishment?.rne || r.rne,
            adminEmail: r.adminContact?.email || r.adminEmail,
            createdAt: r.createdAt,
            updatedAt: r.updatedAt,
          })),
        );
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  const filtered = rows.filter((r) => filter === "all" || r.status === filter);

  return (
    <RequirePlatformMaster redirectTo="/">
      <MarketingShell>
        <main className="mx-auto max-w-5xl px-6 py-10 space-y-6">
          <div>
            <p className="text-sm text-stone-500">
              <Link href="/plateforme" className="text-[#2F6B4A] hover:underline">
                ← Plateforme
              </Link>
            </p>
            <h1 className="mt-2 text-3xl font-black text-[#14231A]">
              Dossiers <span className={SCOLA_GRADIENT_TEXT}>inscription</span>
            </h1>
            <p className="mt-1 text-sm text-stone-600">
              Validation Microsoft Education, paiement Easytransac et provisioning.
            </p>
          </div>

          <div className="flex flex-wrap gap-2">
            {FILTERS.map((f) => (
              <button
                key={f}
                type="button"
                onClick={() => setFilter(f)}
                className={`rounded-full px-3 py-1 text-xs font-bold border ${
                  filter === f
                    ? "border-[#2F6B4A] bg-emerald-50 text-[#2F6B4A]"
                    : "border-stone-200 text-stone-600"
                }`}
              >
                {f === "all" ? "Tous" : SIGNUP_STATUS_LABELS[f]}
              </button>
            ))}
          </div>

          {loading ? (
            <p className="text-sm text-stone-500">Chargement…</p>
          ) : filtered.length === 0 ? (
            <p className="rounded-xl border border-dashed border-stone-300 p-8 text-center text-stone-500 text-sm">
              Aucun dossier pour ce filtre.
            </p>
          ) : (
            <ul className="divide-y divide-stone-100 rounded-2xl border border-stone-200 bg-white/90 overflow-hidden">
              {filtered.map((r) => (
                <li key={r.id}>
                  <Link
                    href={`/plateforme/demandes/${r.id}`}
                    className="flex flex-wrap items-center justify-between gap-3 px-5 py-4 hover:bg-stone-50"
                  >
                    <div>
                      <p className="font-bold text-stone-900">{r.legalName}</p>
                      <p className="text-xs text-stone-500">
                        RNE {r.rne} · {r.adminEmail}
                      </p>
                    </div>
                    <span className="text-xs font-bold rounded-full bg-stone-100 px-3 py-1 text-stone-700">
                      {SIGNUP_STATUS_LABELS[r.status]}
                    </span>
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </main>
      </MarketingShell>
    </RequirePlatformMaster>
  );
}
