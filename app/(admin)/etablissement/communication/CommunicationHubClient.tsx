"use client";

import { useCallback, useEffect, useState } from "react";
import dynamic from "next/dynamic";
import RequireOrgAdmin from "@/app/components/RequireOrgAdmin";
import CommunicationDocumentsPanel from "@/app/components/communication/CommunicationDocumentsPanel";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ModuleTabFallback from "@/app/components/module-chrome/ModuleTabFallback";
import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";
import type { ToolboxConfig } from "@/app/lib/toolbox-types";

type Tab = "documents" | "tarifs" | "actus";

const CommunicationSitePostsPanel = dynamic(
  () => import("@/app/components/communication/CommunicationSitePostsPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const CommunicationTarifsPanel = dynamic(
  () => import("@/app/components/communication/CommunicationTarifsPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);

export default function CommunicationHubClient() {
  const [tab, setTab] = useState<Tab>("documents");
  const [config, setConfig] = useState<ToolboxConfig | null>(null);
  const [publicOrigin, setPublicOrigin] = useState("");
  const [customWebsiteEnabled, setCustomWebsiteEnabled] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [tbRes, setRes] = await Promise.all([
        fetch("/api/toolbox/config", { cache: "no-store" }),
        fetch("/api/settings", { cache: "no-store" }),
      ]);
      const tj = await tbRes.json();
      if (!tbRes.ok) throw new Error(tj.error || "Erreur toolbox");
      setConfig(tj.config);
      setPublicOrigin(typeof tj.publicOrigin === "string" ? tj.publicOrigin.replace(/\/$/, "") : "");

      if (setRes.ok) {
        const sj = await setRes.json();
        setCustomWebsiteEnabled(Boolean(sj.config?.identity?.customWebsite?.enabled));
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

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

  function patchTarifs(patch: Partial<ToolboxConfig["tools"]["simulateur-tarifs"]>) {
    if (!config) return;
    setConfig({
      ...config,
      tools: {
        ...config.tools,
        "simulateur-tarifs": { ...config.tools["simulateur-tarifs"], ...patch },
      },
    });
  }

  if (loading) {
    return (
      <RequireOrgAdmin>
        <ModulePageShell maxWidthClass="max-w-[1280px]">
          <p className="text-slate-500">Chargement…</p>
        </ModulePageShell>
      </RequireOrgAdmin>
    );
  }

  const tarifs = config?.tools["simulateur-tarifs"];

  return (
    <RequireOrgAdmin>
      <ModulePageShell maxWidthClass="max-w-[1280px]">
        <ModulePageHeader
          eyebrow="Établissement"
          title="Communication"
          description="Documents familles (PDF zéro papier), simulateur de tarifs, et actus site si vitrine Scola activée."
          actions={
            tab === "tarifs" ? (
              <ModuleButton onClick={() => void save()} disabled={saving || !config}>
                {saving ? "Enregistrement…" : "Enregistrer"}
              </ModuleButton>
            ) : null
          }
        />

        <ModuleTabNav
          className="mb-6"
          tabs={[
            { id: "documents", label: "Documents" },
            { id: "tarifs", label: "Tarifs" },
            {
              id: "actus",
              label: customWebsiteEnabled ? "Actus site" : "Actus site (option)",
            },
          ]}
          active={tab}
          onChange={setTab}
        />

        {error ? (
          <p className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
            {error}
          </p>
        ) : null}
        {msg ? (
          <p className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">
            {msg}
          </p>
        ) : null}

        {tab === "documents" ? <CommunicationDocumentsPanel /> : null}

        {tab === "actus" ? (
          <CommunicationSitePostsPanel enabled={customWebsiteEnabled} onRefreshFlag={() => void load()} />
        ) : null}

        {tab === "tarifs" ? (
          <CommunicationTarifsPanel tarifs={tarifs} publicOrigin={publicOrigin} onPatch={patchTarifs} />
        ) : null}
      </ModulePageShell>
    </RequireOrgAdmin>
  );
}
