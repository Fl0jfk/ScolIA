"use client";

import type { ReactNode } from "react";
import StageClassRosterPanel from "@/app/components/stages/StageClassRosterPanel";

export default function StagesClassePanel({
  onOpenConvention,
  selectedConventionId,
  canFileOneDrive,
  oneDriveConnected,
  onFileOneDrive,
  filingConventionId,
  detailPanel,
}: {
  onOpenConvention: (id: string) => void;
  selectedConventionId?: string | null;
  canFileOneDrive: boolean;
  oneDriveConnected: boolean;
  onFileOneDrive: (id: string) => void;
  filingConventionId: string | null;
  detailPanel?: ReactNode;
}) {
  return (
    <section data-tour="stages-classe" className="space-y-6">
      <div className="rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-bold text-[#1F3D2B]">Suivi des stages par classe</h2>
        <p className="mt-2 text-sm text-stone-600 max-w-3xl">
          Cliquez sur un élève pour voir toutes ses conventions (statuts, signatures, référent).
          Ouvrez un dossier pour valider, relancer ou suivre les signatures — tout est ici.
        </p>
        <div className="mt-6">
          <StageClassRosterPanel
            onOpenConvention={onOpenConvention}
            selectedConventionId={selectedConventionId}
            canFileOneDrive={canFileOneDrive}
            oneDriveConnected={oneDriveConnected}
            onFileOneDrive={onFileOneDrive}
            filingConventionId={filingConventionId}
          />
        </div>
      </div>
      {detailPanel}
    </section>
  );
}
