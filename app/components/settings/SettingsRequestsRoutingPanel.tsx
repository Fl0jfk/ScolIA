"use client";

import type { Dispatch, SetStateAction } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import type { ClerkMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import RequestsRoutingEditor from "@/app/components/settings/RequestsRoutingEditor";
import type { RequestsRoutingConfig } from "@/app/lib/app-config-schemas";

export default function SettingsRequestsRoutingPanel({
  requestsRouting,
  setRequestsRouting,
  clerkMembers,
  membersLoading,
  saving,
  onSave,
}: {
  requestsRouting: RequestsRoutingConfig | null;
  setRequestsRouting: Dispatch<SetStateAction<RequestsRoutingConfig | null>>;
  clerkMembers: ClerkMemberOption[];
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
        members={clerkMembers}
        membersLoading={membersLoading}
      />
      <ModuleButton variant="primary" disabled={saving} onClick={onSave}>
        Enregistrer le routage des demandes
      </ModuleButton>
    </div>
  );
}
