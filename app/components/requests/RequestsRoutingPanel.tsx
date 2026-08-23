"use client";

import RequestsRoutingEditor from "@/app/components/settings/RequestsRoutingEditor";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import type { RequestsRoutingConfig } from "@/app/lib/app-config-schemas";

type Member = { externalUserId: string; email: string; displayName: string };

export default function RequestsRoutingPanel({
  requestsRouting,
  onChange,
  members,
  membersLoading,
  routingMsg,
  routingBusy,
  onSave,
}: {
  requestsRouting: RequestsRoutingConfig | null;
  onChange: (config: RequestsRoutingConfig) => void;
  members: Member[];
  membersLoading: boolean;
  routingMsg: string | null;
  routingBusy: boolean;
  onSave: () => void | Promise<void>;
}) {
  return (
    <div className="mt-6 max-w-5xl space-y-4">
      {routingMsg ? <p className="text-sm text-emerald-700">{routingMsg}</p> : null}
      {requestsRouting ? (
        <>
          <RequestsRoutingEditor
            config={requestsRouting}
            onChange={onChange}
            members={members}
            membersLoading={membersLoading}
          />
          <ModuleButton variant="primary" disabled={routingBusy} onClick={() => void onSave()}>
            {routingBusy ? "…" : "Enregistrer le routage"}
          </ModuleButton>
        </>
      ) : (
        <p className="text-sm text-slate-500">Chargement du routage…</p>
      )}
    </div>
  );
}
