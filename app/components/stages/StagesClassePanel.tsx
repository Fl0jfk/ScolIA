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
  canCreateOffline,
  onCreateOffline,
  refreshToken,
}: {
  onOpenConvention: (id: string) => void;
  selectedConventionId?: string | null;
  focusClassName?: string | null;
  detailSlot?: ReactNode;
  canFileOneDrive: boolean;
  oneDriveConnected: boolean;
  onFileOneDrive: (id: string) => void;
  filingConventionId: string | null;
  canCreateOffline?: boolean;
  onCreateOffline?: (preset: {
    firstName: string;
    lastName: string;
    className: string;
    ine?: string;
  }) => void;
  refreshToken?: number;
}) {
  return (
    <section data-tour="stages-classe" className="space-y-3">
      <div>
        <h2 className="text-lg font-bold text-[#1F3D2B]">Suivi des stages par classe</h2>
        <p className="mt-1 text-sm text-stone-600">
          Liste des élèves, statut et signatures. Ouvrez un dossier pour valider ou relancer.
          Les validations quotidiennes se font aussi depuis le tableau de bord.
        </p>
      </div>
      <StageClassRosterPanel
        onOpenConvention={onOpenConvention}
        selectedConventionId={selectedConventionId}
        focusClassName={focusClassName}
        detailSlot={detailSlot}
        canFileOneDrive={canFileOneDrive}
        oneDriveConnected={oneDriveConnected}
        onFileOneDrive={onFileOneDrive}
        filingConventionId={filingConventionId}
        canCreateOffline={canCreateOffline}
        onCreateOffline={onCreateOffline}
        refreshToken={refreshToken}
      />
    </section>
  );
}
