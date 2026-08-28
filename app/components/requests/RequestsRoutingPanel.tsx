"use client";

import RequestsRoutingEditor from "@/app/components/settings/RequestsRoutingEditor";
import RequestOrgEditor from "@/app/components/settings/RequestOrgEditor";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";
import type { RequestsOrgConfig, RequestsRoutingConfig } from "@/app/lib/app-config-schemas";
import { useState } from "react";

type Member = { externalUserId: string; email: string; displayName: string };

type SettingsSubTab = "routing" | "org";

export default function RequestsRoutingPanel({
  requestsRouting,
  onChangeRouting,
  requestsOrg,
  onChangeOrg,
  members,
  membersLoading,
  routingMsg,
  routingBusy,
  onSave,
}: {
  requestsRouting: RequestsRoutingConfig | null;
  onChangeRouting: (config: RequestsRoutingConfig) => void;
  requestsOrg: RequestsOrgConfig | null;
  onChangeOrg: (config: RequestsOrgConfig) => void;
  members: Member[];
  membersLoading: boolean;
  routingMsg: string | null;
  routingBusy: boolean;
  onSave: () => void | Promise<void>;
}) {
  const [subTab, setSubTab] = useState<SettingsSubTab>("org");

  return (
    <div className="mt-6 max-w-5xl space-y-4">
      {routingMsg ? (
        <p className={`text-sm ${routingMsg.includes("Erreur") || routingMsg.includes("Échec") ? "text-rose-600" : "text-emerald-700"}`}>
          {routingMsg}
        </p>
      ) : null}

      <ModuleTabNav
        tabs={[
          { id: "org", label: "Organisation services" },
          { id: "routing", label: "Routage IA & files" },
        ]}
        active={subTab}
        onChange={(id) => setSubTab(id as SettingsSubTab)}
      />

      {subTab === "org" ? (
        requestsOrg && requestsRouting ? (
          <RequestOrgEditor
            org={requestsOrg}
            routing={requestsRouting}
            onChange={onChangeOrg}
            members={members}
            membersLoading={membersLoading}
          />
        ) : (
          <p className="text-sm text-slate-500">Chargement de l&apos;organisation…</p>
        )
      ) : requestsRouting ? (
        <RequestsRoutingEditor
          config={requestsRouting}
          onChange={onChangeRouting}
          members={members}
          membersLoading={membersLoading}
        />
      ) : (
        <p className="text-sm text-slate-500">Chargement du routage…</p>
      )}

      <ModuleButton variant="primary" disabled={routingBusy || !requestsRouting || !requestsOrg} onClick={() => void onSave()}>
        {routingBusy ? "Enregistrement…" : "Enregistrer les réglages demandes"}
      </ModuleButton>
    </div>
  );
}
