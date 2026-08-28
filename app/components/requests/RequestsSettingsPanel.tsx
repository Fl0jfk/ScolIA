"use client";

import { useState } from "react";
import RequestOrgEditor from "@/app/components/settings/RequestOrgEditor";
import RequestsRoutingEditor from "@/app/components/settings/RequestsRoutingEditor";
import RequestPersonnelTagsEditor from "@/app/components/settings/RequestPersonnelTagsEditor";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";
import type { RequestsOrgConfig, RequestsRoutingConfig } from "@/app/lib/app-config-schemas";
import type { DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";

type SettingsTab = "services" | "files" | "tags" | "options";

type Member = DirectoryMemberOption;

export default function RequestsSettingsPanel({
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
  const [tab, setTab] = useState<SettingsTab>("services");

  return (
    <div className="mt-6 max-w-5xl space-y-4">
      <section className="rounded-2xl border border-slate-200 bg-gradient-to-br from-slate-50 to-indigo-50/30 p-5 space-y-2">
        <h2 className="text-lg font-black text-slate-900">Réglages des demandes</h2>
        <p className="text-sm text-slate-600 leading-relaxed">
          Le routage cible d&apos;abord un <strong>service</strong> (pile des managers), pas une personne.
          Si le service est clair mais la personne ne l&apos;est pas, la demande reste en pile — les managers
          peuvent la prendre ou la confier. La corbeille établissement n&apos;intervient qu&apos;en cas
          d&apos;ambiguïté entre services.
        </p>
        <ol className="text-xs text-slate-500 list-decimal list-inside space-y-0.5">
          <li>Organisez les services, managers et membres</li>
          <li>Définissez les files et leurs mots-clés (IA + fallback local)</li>
          <li>Affinez avec les tags équipe (cycles, compétences)</li>
        </ol>
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
          { id: "files", label: "2. Files & mots-clés" },
          { id: "tags", label: "3. Tags équipe" },
          { id: "options", label: "4. Options" },
        ]}
        active={tab}
        onChange={(id) => setTab(id as SettingsTab)}
      />

      {tab === "services" && requestsOrg && requestsRouting ? (
        <RequestOrgEditor
          org={requestsOrg}
          routing={requestsRouting}
          onChange={onChangeOrg}
          members={members}
          membersLoading={membersLoading}
        />
      ) : null}

      {tab === "files" && requestsRouting ? (
        <RequestsRoutingEditor
          config={requestsRouting}
          onChange={onChangeRouting}
          members={members}
          membersLoading={membersLoading}
          mode="files"
        />
      ) : null}

      {tab === "tags" && requestsRouting ? (
        <RequestPersonnelTagsEditor
          config={requestsRouting}
          onChange={onChangeRouting}
          members={members}
          membersLoading={membersLoading}
        />
      ) : null}

      {tab === "options" && requestsRouting ? (
        <RequestsRoutingEditor
          config={requestsRouting}
          onChange={onChangeRouting}
          members={members}
          membersLoading={membersLoading}
          mode="options"
        />
      ) : null}

      {!requestsRouting || !requestsOrg ? (
        <p className="text-sm text-slate-500">Chargement des réglages…</p>
      ) : null}

      <ModuleButton
        variant="primary"
        disabled={routingBusy || !requestsRouting || !requestsOrg}
        onClick={() => void onSave()}
      >
        {routingBusy ? "Enregistrement…" : "Enregistrer tous les réglages"}
      </ModuleButton>
    </div>
  );
}
