"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import RequireOrgAdmin from "@/app/components/RequireOrgAdmin";
import ToolboxModal from "@/app/components/toolbox/ToolboxModal";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { dash } from "@/app/lib/dashboard-brand";
import {
  renderToolboxAdminIcon,
  renderToolboxHubIcon,
  renderToolboxIcon,
} from "@/app/components/toolbox/ToolboxIcons";
import type { ToolboxConfig } from "@/app/lib/toolbox-types";
import {
  TOOLBOX_ADMIN_LINKS,
  TOOLBOX_HUB_LINKS,
  TOOLBOX_TOOLS_META,
} from "@/app/lib/toolbox-tools";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";

function Toggle({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
}) {
  return (
    <label className="flex items-center gap-3 cursor-pointer">
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} className="h-4 w-4" />
      <span className={`text-sm font-semibold ${dash.ink}`}>{label}</span>
    </label>
  );
}

export default function ToolboxAdminPage() {
  const isOrgAdmin = useIsOrgAdmin();
  const [config, setConfig] = useState<ToolboxConfig | null>(null);
  const [publicOrigin, setPublicOrigin] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [previewOpen, setPreviewOpen] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/toolbox/config", { cache: "no-store" });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      setConfig(j.config);
      setPublicOrigin(typeof j.publicOrigin === "string" ? j.publicOrigin.replace(/\/$/, "") : "");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (isOrgAdmin) void load();
    else setLoading(false);
  }, [load, isOrgAdmin]);

  useEffect(() => {
    const t = new URLSearchParams(window.location.search).get("tab");
    if (t === "rentree" || t === "portes-ouvertes" || t === "secret-santa") {
      window.location.replace(`/etablissement/evenements?tab=${t}`);
    }
    if (t === "fournitures") {
      window.location.replace("/etablissement/evenements?tab=rentree");
    }
    if (t === "tarifs") {
      window.location.replace("/etablissement/communication");
    }
  }, []);

  async function save() {
    if (!config) return;
    setSaving(true);
    setMsg(null);
    setError(null);
    try {
      const res = await fetch("/api/toolbox/config", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(config),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      setConfig(j.config);
      setMsg("Configuration enregistrée.");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  }

  function patchTool<K extends keyof ToolboxConfig["tools"]>(
    key: K,
    patch: Partial<ToolboxConfig["tools"][K]>,
  ) {
    if (!config) return;
    setConfig({
      ...config,
      tools: {
        ...config.tools,
        [key]: { ...config.tools[key], ...patch },
      },
    });
  }

  const qrEnabled = config?.tools.qrcreator.enabled !== false;
  const repartitionEnabled =
    config && "enabled" in config.tools["repartition-classes"]
      ? config.tools["repartition-classes"].enabled
      : false;
  const covoiturageEnabled = config?.tools.covoiturage?.enabled === true;

  return (
    <ModulePageShell maxWidthClass="max-w-[1280px]">
      <ModulePageHeader
        eyebrow="Services"
        title="Boîte à outils"
        description="QR code, photocopies et petits utilitaires activables (covoiturage, répartition…)."
      />

      {error && (
        <p className="mb-4 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
          {error}
        </p>
      )}

      <section className="mb-10 grid gap-4 sm:grid-cols-2">
        <Link href="/qrcreator">
          <ModuleCard bodyClassName="p-5 transition hover:-translate-y-0.5">
            <div className="flex items-start gap-4">
              <span className={`flex h-14 w-14 shrink-0 items-center justify-center rounded-2xl ${dash.bgSoft} ${dash.ink}`}>
                {renderToolboxIcon("qrcreator", "w-8 h-8")}
              </span>
              <div>
                <h2 className={`font-semibold ${dash.ink}`}>QR Code</h2>
                <p className={`mt-1 text-xs ${dash.textMid}`}>
                  Créer un QR code personnalisé avec le logo de l&apos;établissement.
                </p>
                <span className={`mt-2 inline-block text-xs font-semibold underline ${dash.textPrimary}`}>
                  Ouvrir →
                </span>
              </div>
            </div>
          </ModuleCard>
        </Link>

        {TOOLBOX_HUB_LINKS.map((link) => (
          <Link key={link.id} href={link.adminPath}>
            <ModuleCard bodyClassName="p-5 transition hover:-translate-y-0.5">
              <div className="flex items-start gap-4">
                <span
                  className={`flex h-14 w-14 shrink-0 items-center justify-center rounded-2xl ${link.bg} ${link.color}`}
                >
                  {renderToolboxHubIcon(link.id, "w-8 h-8")}
                </span>
                <div>
                  <h2 className={`font-semibold ${dash.ink}`}>{link.label}</h2>
                  <p className={`mt-1 text-xs ${dash.textMid}`}>{link.description}</p>
                  <span className={`mt-2 inline-block text-xs font-semibold underline ${dash.textPrimary}`}>
                    Ouvrir →
                  </span>
                </div>
              </div>
            </ModuleCard>
          </Link>
        ))}

        {(repartitionEnabled || isOrgAdmin) && (
          <Link href="/toolbox/repartition-classes" className="sm:col-span-2">
            <ModuleCard bodyClassName="p-5 transition hover:-translate-y-0.5">
              <div className="flex items-start gap-4">
                <span className={`flex h-14 w-14 shrink-0 items-center justify-center rounded-2xl ${dash.bgSoft} ${dash.textPrimary}`}>
                  {renderToolboxIcon("repartition-classes", "w-8 h-8")}
                </span>
                <div>
                  <h2 className={`font-semibold ${dash.ink}`}>Répartition des classes</h2>
                  <p className={`mt-1 text-xs ${dash.textMid}`}>
                    Préparer la classe, vœux parents et moteur de répartition.
                  </p>
                  <span className={`mt-2 inline-block text-xs font-semibold underline ${dash.textPrimary}`}>
                    Ouvrir →
                  </span>
                </div>
              </div>
            </ModuleCard>
          </Link>
        )}

        {(covoiturageEnabled || isOrgAdmin) && (
          <Link href="/covoiturage">
            <ModuleCard bodyClassName="p-5 transition hover:-translate-y-0.5">
              <div className="flex items-start gap-4">
                <span className="flex h-14 w-14 shrink-0 items-center justify-center rounded-2xl bg-emerald-50 text-emerald-800">
                  {renderToolboxIcon("covoiturage", "w-8 h-8")}
                </span>
                <div>
                  <h2 className={`font-semibold ${dash.ink}`}>Covoiturage</h2>
                  <p className={`mt-1 text-xs ${dash.textMid}`}>
                    {covoiturageEnabled
                      ? "Mise en relation familles pour les trajets quotidiens."
                      : "Désactivé — activez l’outil ci-dessous quand il sera prêt."}
                  </p>
                  <span className={`mt-2 inline-block text-xs font-semibold underline ${dash.textPrimary}`}>
                    {covoiturageEnabled ? "Ouvrir →" : "Aperçu →"}
                  </span>
                </div>
              </div>
            </ModuleCard>
          </Link>
        )}
      </section>

      {isOrgAdmin ? (
        <RequireOrgAdmin>
          <div className={`border-t pt-8 ${dash.divider}`}>
            <header className="mb-6 flex flex-wrap items-start justify-between gap-4">
              <div>
                <p className={dash.fieldLabel}>Administration</p>
                <h2 className={`text-xl font-semibold ${dash.ink}`}>Activation des outils</h2>
                <p className={`mt-1 max-w-xl text-sm ${dash.textMid}`}>
                  Rentrée / fournitures →{" "}
                  <Link href="/etablissement/evenements" className={`font-semibold underline ${dash.ink}`}>
                    Événements
                  </Link>
                  . Tarifs →{" "}
                  <Link href="/etablissement/communication" className={`font-semibold underline ${dash.ink}`}>
                    Communication
                  </Link>
                  .
                </p>
              </div>
              <div className="flex flex-wrap gap-2">
                <ModuleButton variant="secondary" onClick={() => setPreviewOpen(true)}>
                  Aperçu modal
                </ModuleButton>
                <ModuleButton onClick={() => void save()} disabled={saving || loading || !config}>
                  {saving ? "Enregistrement…" : "Enregistrer"}
                </ModuleButton>
              </div>
            </header>

            {msg && (
              <p className="mb-4 rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">
                {msg}
              </p>
            )}

            {loading || !config ? (
              <p className={`text-sm ${dash.textMid}`}>Chargement de la configuration…</p>
            ) : (
              <section className="grid gap-4 sm:grid-cols-2">
                {TOOLBOX_TOOLS_META.map((meta) => {
                  const tool = config.tools[meta.id];
                  const enabled =
                    meta.id === "qrcreator"
                      ? qrEnabled
                      : "enabled" in tool
                        ? tool.enabled
                        : false;
                  return (
                    <ModuleCard key={meta.id} bodyClassName="p-5">
                      <div className="flex items-start gap-4">
                        <span
                          className={`flex h-14 w-14 shrink-0 items-center justify-center rounded-2xl ${meta.bg} ${meta.color}`}
                        >
                          {renderToolboxIcon(meta.id, "w-8 h-8")}
                        </span>
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center justify-between gap-2">
                            <h3 className={`font-semibold ${dash.ink}`}>{meta.label}</h3>
                            <Toggle
                              checked={enabled}
                              onChange={(v) => {
                                if (meta.id === "qrcreator") patchTool("qrcreator", { enabled: v });
                                else patchTool(meta.id, { enabled: v } as never);
                              }}
                              label=""
                            />
                          </div>
                          <p className={`mt-1 text-xs ${dash.textMid}`}>{meta.description}</p>
                          {meta.publicPath && enabled && (
                            <a
                              href={`${publicOrigin}${meta.publicPath}`}
                              target="_blank"
                              rel="noopener noreferrer"
                              className={`mt-2 inline-block break-all text-xs font-semibold underline ${dash.textPrimary}`}
                            >
                              Page publique → {publicOrigin}
                              {meta.publicPath}
                            </a>
                          )}
                        </div>
                      </div>
                    </ModuleCard>
                  );
                })}

                <div className="sm:col-span-2">
                  <h3 className={`mb-3 ${dash.fieldLabel}`}>Raccourcis admin</h3>
                  <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                    {TOOLBOX_ADMIN_LINKS.map((link) => (
                      <Link key={link.id} href={link.adminPath}>
                        <ModuleCard bodyClassName="p-5 transition hover:-translate-y-0.5">
                          <div className="flex items-start gap-4">
                            <span
                              className={`flex h-14 w-14 shrink-0 items-center justify-center rounded-2xl ${link.bg} ${link.color}`}
                            >
                              {renderToolboxAdminIcon(link.id, "w-8 h-8")}
                            </span>
                            <div className="min-w-0">
                              <h4 className={`font-semibold ${dash.ink}`}>{link.label}</h4>
                              <p className={`mt-1 text-xs ${dash.textMid}`}>{link.description}</p>
                            </div>
                          </div>
                        </ModuleCard>
                      </Link>
                    ))}
                  </div>
                </div>
              </section>
            )}

            <ToolboxModal open={previewOpen} onClose={() => setPreviewOpen(false)} />
          </div>
        </RequireOrgAdmin>
      ) : null}
    </ModulePageShell>
  );
}
