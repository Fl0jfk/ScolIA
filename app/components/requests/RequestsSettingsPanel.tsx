"use client";

import { useState } from "react";
import RequestOrgEditor from "@/app/components/settings/RequestOrgEditor";
import RequestsRoutingEditor from "@/app/components/settings/RequestsRoutingEditor";
import RequestPersonnelTagsEditor from "@/app/components/settings/RequestPersonnelTagsEditor";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";
import type { RequestsOrgConfig, RequestsRoutingConfig } from "@/app/lib/app-config-schemas";
import { defaultRequestsOrg } from "@/app/lib/requests-org-shared";
import { defaultRequestsRouting } from "@/app/lib/requests-routing-defaults";
import type { DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";

type SettingsTab = "services" | "personnes" | "options";

type Member = DirectoryMemberOption;

export default function RequestsSettingsPanel({
  requestsRouting,
  onChangeRouting,
  requestsOrg,
  onChangeOrg,
  members,
  membersLoading,
  settingsLoading,
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
  settingsLoading: boolean;
  routingMsg: string | null;
  routingBusy: boolean;
  onSave: () => void | Promise<void>;
}) {
  const [tab, setTab] = useState<SettingsTab>("services");

  const orgReady = Boolean(requestsOrg);
  const routingReady = Boolean(requestsRouting);
  const orgConfig = requestsOrg ?? defaultRequestsOrg();
  const routingConfig = requestsRouting ?? defaultRequestsRouting();

  return (
    <div className="mt-6 max-w-5xl space-y-4">
      <section className="rounded-2xl border border-slate-200 bg-gradient-to-br from-slate-50 to-indigo-50/30 p-5 space-y-2">
        <h2 className="text-lg font-black text-slate-900">Réglages des demandes</h2>
        <p className="text-sm text-slate-600 leading-relaxed">
          Trois étapes simples : créez vos <strong>services</strong> avec leurs tags métier et affectez
          managers et membres ; précisez ensuite les <strong>compétences par personne</strong> ; configurez
          les <strong>options</strong> (page parents, règle direction).
        </p>
        <p className="text-xs text-slate-500">
          Les demandes « pour la direction » passent toujours d&apos;abord par l&apos;administratif — la
          direction ne reçoit qu&apos;un transfert manuel validé.
        </p>
      </section>

      {routingMsg ? (
        <p
          className={`text-sm ${routingMsg.includes("Erreur") || routingMsg.includes("Échec") ? "text-rose-600" : "text-emerald-700"}`}
        >
          {routingMsg}
        </p>
      ) : null}

      <ModuleTabNav
        tabs={[
          { id: "services", label: "1. Services" },
          { id: "personnes", label: "2. Personnes" },
          { id: "options", label: "3. Options" },
        ]}
        active={tab}
        onChange={(id) => setTab(id as SettingsTab)}
      />

      {tab === "services" ? (
        settingsLoading && !orgReady ? (
          <p className="text-sm text-slate-500">Chargement des services…</p>
        ) : (
          <RequestOrgEditor
            org={orgConfig}
            routing={routingConfig}
            onChange={onChangeOrg}
            members={members}
            membersLoading={membersLoading}
          />
        )
      ) : null}

      {tab === "personnes" ? (
        settingsLoading && !routingReady ? (
          <p className="text-sm text-slate-500">Chargement du personnel…</p>
        ) : (
          <RequestPersonnelTagsEditor
            config={routingConfig}
            org={orgConfig}
            onChange={onChangeRouting}
            members={members}
            membersLoading={membersLoading}
          />
        )
      ) : null}

      {tab === "options" ? (
        settingsLoading && !routingReady ? (
          <p className="text-sm text-slate-500">Chargement des options…</p>
        ) : (
          <RequestsRoutingEditor
            config={routingConfig}
            onChange={onChangeRouting}
            members={members}
            membersLoading={membersLoading}
            mode="options"
          />
        )
      ) : null}

      <ModuleButton
        variant="primary"
        disabled={routingBusy || settingsLoading}
        onClick={() => void onSave()}
      >
        {routingBusy ? "Enregistrement…" : "Enregistrer tous les réglages"}
      </ModuleButton>
    </div>
  );
}
