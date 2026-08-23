"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import MarketingShell from "@/app/components/landing/MarketingShell";
import PlatformQuickTenantWizard from "@/app/components/platform/PlatformQuickTenantWizard";
import RequirePlatformMaster from "@/app/components/RequirePlatformMaster";
import { SCOLA_GRADIENT_TEXT } from "@/app/lib/marketing-theme";

type TenantRow = {
  slug: string;
  label: string;
  hostnames: string[];
  appUrl: string;
  configured: { secretKey: boolean };
};

type SetupPayload = {
  writable: boolean;
  tenants: TenantRow[];
};

export default function PlateformePage() {
  const [data, setData] = useState<SetupPayload | null>(null);
  const [loading, setLoading] = useState(true);
  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch("/api/platform/setup", { cache: "no-store" });
      const j = await res.json();
      if (res.ok) setData({ writable: j.writable, tenants: j.tenants });
    } finally {
      setLoading(false);
    }
  }, []);
  useEffect(() => {
    reload();
  }, [reload]);
  return (
    <RequirePlatformMaster redirectTo="/">
      <MarketingShell>
        <main className="mx-auto max-w-4xl px-6 py-10 space-y-10">
          <div>
            <p className="text-sm text-stone-500">
              <Link href="/" className="text-[#2F6B4A] hover:underline">
                ← scola.fr
              </Link>
            </p>
            <h1 className="mt-2 text-3xl font-black text-[#14231A]">
              Espace <span className={SCOLA_GRADIENT_TEXT}>plateforme</span>
            </h1>
            <p className="mt-2 max-w-xl text-sm text-stone-600">
              Créez un nouvel établissement : sous-domaine, bucket S3 et clés auth legacy. Réservé au
              profil Master.
            </p>
            <p className="mt-3">
              <Link
                href="/plateforme/demandes"
                className="inline-flex items-center gap-1 text-sm font-semibold text-[#2F6B4A] hover:underline"
              >
                Dossiers d&apos;inscription →
              </Link>
            </p>
          </div>
          <PlatformQuickTenantWizard writable={data?.writable ?? false} onCreated={reload} />
          <section className="rounded-2xl border border-stone-200 bg-white/80 p-5">
            <div className="flex items-center justify-between gap-3">
              <h2 className="font-bold text-[#14231A]">
                Établissements actifs ({data?.tenants.length ?? "…"})
              </h2>
              <Link
                href="/platform/setup"
                className="text-sm font-semibold text-violet-700 hover:underline"
              >
                Console avancée →
              </Link>
            </div>
            {loading ? (
              <p className="mt-4 text-sm text-stone-500">Chargement…</p>
            ) : (
              <ul className="mt-4 divide-y divide-stone-100">
                {(data?.tenants ?? [])
                  .filter((t) => t.slug !== "platform")
                  .map((t) => (
                    <li key={t.slug} className="flex flex-wrap items-center justify-between gap-2 py-3 text-sm">
                      <div>
                        <p className="font-semibold text-stone-900">{t.label}</p>
                        <p className="font-mono text-xs text-stone-500">
                          {t.hostnames.filter((h) => h !== "localhost").join(", ") || t.slug}
                        </p>
                      </div>
                      <div className="flex items-center gap-3">
                        <span
                          className={`text-xs ${t.configured.secretKey ? "text-emerald-700" : "text-amber-700"}`}
                        >
                          {t.configured.secretKey ? "Auth OK" : "Auth manquante"}
                        </span>
                        {t.appUrl && (
                          <a
                            href={t.appUrl}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-xs font-semibold text-[#2F6B4A] hover:underline"
                          >
                            Ouvrir →
                          </a>
                        )}
                        <Link
                          href={`/plateforme/tenants/${t.slug}/billing`}
                          className="text-xs font-semibold text-amber-800 hover:underline"
                        >
                          Facturation
                        </Link>
                      </div>
                    </li>
                  ))}
              </ul>
            )}
          </section>
          <section className="rounded-xl border border-stone-200 bg-stone-50/80 p-4 text-xs text-stone-600 space-y-2">
            <p className="font-bold text-stone-800">Après la création</p>
            <ol className="list-decimal list-inside space-y-1">
              <li>Pointer le DNS du sous-domaine vers l&apos;application (Scaleway / hébergeur).</li>
              <li>
                Se connecter sur le nouveau domaine avec un compte <code>admin</code> de
                l&apos;établissement.
              </li>
              <li>L&apos;assistant de configuration guide l&apos;admin (identité, modules, etc.).</li>
            </ol>
          </section>
        </main>
      </MarketingShell>
    </RequirePlatformMaster>
  );
}
