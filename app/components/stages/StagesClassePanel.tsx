"use client";

import type { ReactNode } from "react";
import StageClassRosterPanel from "@/app/components/stages/StageClassRosterPanel";

export default function StagesClassePanel({
  onOpenConvention,
  selectedConventionId,
  focusClassName,
  detailSlot,
  canFileOneDrive,
  oneDriveConnected,
  onFileOneDrive,
  filingConventionId,
}: {
  onOpenConvention: (id: string) => void;
  selectedConventionId?: string | null;
  focusClassName?: string | null;
  detailSlot?: ReactNode;
  canFileOneDrive: boolean;
  oneDriveConnected: boolean;
  onFileOneDrive: (id: string) => void;
  filingConventionId: string | null;
}) {
  return (
    <section data-tour="stages-classe" className="space-y-6">
      <div className="rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-bold text-[#1F3D2B]">Suivi des stages par classe</h2>
        <p className="mt-2 text-sm text-stone-600 max-w-3xl">
          Cliquez sur un élève pour voir toutes ses conventions (statuts, signatures, référent).
          Ouvrez un dossier pour valider, relancer ou suivre les signatures — le dossier s&apos;affiche
          directement sous l&apos;élève.
        </p>
        <div className="mt-6">
          <StageClassRosterPanel
            onOpenConvention={onOpenConvention}
            selectedConventionId={selectedConventionId}
            focusClassName={focusClassName}
            detailSlot={detailSlot}
            canFileOneDrive={canFileOneDrive}
            oneDriveConnected={oneDriveConnected}
            onFileOneDrive={onFileOneDrive}
            filingConventionId={filingConventionId}
          />
        </div>
      </div>
    </section>
  );
}
