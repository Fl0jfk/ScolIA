"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import RequirePlatformMaster from "@/app/components/RequirePlatformMaster";
import PlatformTenantEditor from "@/app/components/platform/PlatformTenantEditor";

type TenantRow = {
  slug: string;
  kind: string;
  label: string;
  hostnames: string[];
  appUrl: string;
  dataBucket: string;
  publishableKey: string;
  configured: {
    secretKey: boolean;
    legacyDevKeys: boolean;
    mistral: boolean;
    smtp: boolean;
    microsoft: boolean;
    aws: boolean;
  };
};

type PlatformSetupPayload = {
  multiTenant: boolean;
  writable: boolean;
  registry: {
    bucket: string | null;
    indexKey: string;
    secretsPrefix: string;
    inlineIndex: boolean;
  };
  currentTenantSlug: string;
  tenants: TenantRow[];
  localDev: { legacyEnvOverride: boolean };
};

function ConfigDot({ ok }: { ok: boolean }) {
  return (
    <span
      className={`inline-block h-2 w-2 rounded-full ${ok ? "bg-emerald-500" : "bg-slate-300"}`}
      title={ok ? "Configuré" : "Non configuré"}
    />
  );
}

export default function PlatformSetupPage() {
  const [data, setData] = useState<PlatformSetupPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [editor, setEditor] = useState<{ mode: "create" | "edit"; slug?: string } | null>(null);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/platform/setup", { cache: "no-store" });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      setData(j as PlatformSetupPayload);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  const onSaved = () => {
    setEditor(null);
    reload();
  };

  return (
    <RequirePlatformMaster>
      <div className="max-w-5xl mx-auto p-6 space-y-6">
        <div className="space-y-2">
          <p className="text-sm text-slate-500">
            <Link href="/parametres" className="text-indigo-600 hover:underline">
              ← Paramètres
            </Link>
          </p>
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <h1 className="text-2xl font-bold text-slate-900">Configuration plateforme</h1>
              <p className="text-sm text-slate-600 mt-1 max-w-2xl">
                Gestion du registry multi-tenant, clés auth legacy et intégrations. Profil Master uniquement.
              </p>
            </div>
            {data?.writable && (
              <button
                type="button"
                onClick={() => setEditor({ mode: "create" })}
                className="rounded-lg bg-violet-700 px-4 py-2 text-sm font-medium text-white hover:bg-violet-800"
              >
                + Nouveau tenant
              </button>
            )}
          </div>
        </div>

        {loading && !data && <p className="text-slate-500">Chargement…</p>}
        {error && <p className="text-red-600 text-sm">{error}</p>}

        {data && (
          <>
            <section className="rounded-xl border border-slate-200 bg-white p-5 space-y-3">
              <h2 className="font-semibold text-slate-900">Registry S3</h2>
              <dl className="grid gap-2 text-sm sm:grid-cols-2">
                <div>
                  <dt className="text-slate-500">Multi-tenant</dt>
                  <dd>{data.multiTenant ? "Activé" : "Mono-tenant"}</dd>
                </div>
                <div>
                  <dt className="text-slate-500">Écriture</dt>
                  <dd>{data.writable ? "Activée" : "Lecture seule"}</dd>
                </div>
                <div>
                  <dt className="text-slate-500">Bucket</dt>
                  <dd className="font-mono text-xs">{data.registry.bucket || "—"}</dd>
                </div>
                <div>
                  <dt className="text-slate-500">Index / secrets</dt>
                  <dd className="font-mono text-xs">
                    {data.registry.indexKey} · {data.registry.secretsPrefix}/{"{slug}"}.json
                  </dd>
                </div>
              </dl>
              {!data.writable && (
                <p className="text-xs text-amber-800 bg-amber-50 border border-amber-200 rounded-lg px-3 py-2">
                  Pour éditer depuis l&apos;interface, définissez <code>REGISTRY_BUCKET</code> (et les
                  clés IAM plateforme) sur Scaleway / l&apos;environnement de production.
                </p>
              )}
              {data.localDev.legacyEnvOverride && (
                <p className="text-xs text-amber-800 bg-amber-50 border border-amber-200 rounded-lg px-3 py-2">
                  Overriauth legacy local via <code>.env.local</code> actif (prioritaire en dev).
                </p>
              )}
            </section>

            {editor && (
              <PlatformTenantEditor
                mode={editor.mode}
                slug={editor.slug}
                writable={data.writable}
                onClose={() => setEditor(null)}
                onSaved={onSaved}
              />
            )}

            <section className="rounded-xl border border-slate-200 bg-white overflow-hidden">
              <h2 className="font-semibold text-slate-900 px-5 pt-5 pb-2">
                Tenants ({data.tenants.length})
              </h2>
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead className="bg-slate-50 text-left text-slate-600">
                    <tr>
                      <th className="px-4 py-2 font-medium">Établissement</th>
                      <th className="px-4 py-2 font-medium">Hostnames</th>
                      <th className="px-4 py-2 font-medium">Intégrations</th>
                      <th className="px-4 py-2 font-medium w-28" />
                    </tr>
                  </thead>
                  <tbody>
                    {data.tenants.map((t) => (
                      <tr
                        key={t.slug}
                        className={`border-t border-slate-100 ${
                          t.slug === data.currentTenantSlug ? "bg-indigo-50/40" : ""
                        }`}
                      >
                        <td className="px-4 py-3">
                          <div className="font-medium text-slate-900">
                            {t.label}
                            {t.slug === data.currentTenantSlug && (
                              <span className="ml-2 text-xs font-normal text-indigo-600">
                                (courant)
                              </span>
                            )}
                          </div>
                          <div className="text-xs text-slate-500 font-mono">{t.slug}</div>
                          <div className="text-xs text-slate-400 font-mono mt-0.5">{t.dataBucket}</div>
                        </td>
                        <td className="px-4 py-3 text-xs">{t.hostnames.join(", ") || "—"}</td>
                        <td className="px-4 py-3">
                          <div className="flex flex-wrap gap-2 items-center text-xs text-slate-600">
                            <span title="Secret key"><ConfigDot ok={t.configured.secretKey} /> Auth</span>
                            <span title="Auth dev"><ConfigDot ok={t.configured.legacyDevKeys} /> Dev</span>
                            <span title="AWS"><ConfigDot ok={t.configured.aws} /> AWS</span>
                            <span title="SMTP"><ConfigDot ok={t.configured.smtp} /> SMTP</span>
                            <span title="Mistral"><ConfigDot ok={t.configured.mistral} /> Mistral</span>
                            <span title="Microsoft"><ConfigDot ok={t.configured.microsoft} /> MS</span>
                          </div>
                        </td>
                        <td className="px-4 py-3">
                          <button
                            type="button"
                            onClick={() => setEditor({ mode: "edit", slug: t.slug })}
                            className="text-sm text-violet-700 font-medium hover:underline"
                          >
                            Modifier
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </section>
          </>
        )}
      </div>
    </RequirePlatformMaster>
  );
}
