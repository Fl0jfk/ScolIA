"use client";

import type { Dispatch, SetStateAction } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import type { DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import RequestsRoutingEditor from "@/app/components/settings/RequestsRoutingEditor";
import type { RequestsRoutingConfig } from "@/app/lib/app-config-schemas";

export default function SettingsRequestsRoutingPanel({
  requestsRouting,
  setRequestsRouting,
  directoryMembers,
  membersLoading,
  saving,
  onSave,
}: {
  requestsRouting: RequestsRoutingConfig | null;
  setRequestsRouting: Dispatch<SetStateAction<RequestsRoutingConfig | null>>;
  directoryMembers: DirectoryMemberOption[];
  membersLoading: boolean;
  saving: boolean;
  onSave: () => void;
}) {
  if (!requestsRouting) {
    return <p className="text-slate-500 text-sm">Chargement du catalogue de routage…</p>;
  }

  return (
    <div className="space-y-4">
      <RequestsRoutingEditor
        config={requestsRouting}
        onChange={(next) => setRequestsRouting(next)}
        members={directoryMembers}
        membersLoading={membersLoading}
      />
      <ModuleButton variant="primary" disabled={saving} onClick={onSave}>
        Enregistrer le routage des demandes
      </ModuleButton>
    </div>
  );
}
