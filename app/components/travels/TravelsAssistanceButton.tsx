"use client";

import { TripButton } from "@/app/components/travels/TripDetailUI";
import type { TravelsAssistanceCardApiStatus } from "@/app/lib/travels-assistance-card-shared";

export function TravelsAssistanceButton({
  status,
  size = "md",
  className = "",
}: {
  status: TravelsAssistanceCardApiStatus | null | undefined;
  size?: "sm" | "md" | "lg";
  className?: string;
}) {
  if (!status?.configured || !status.downloadUrl) return null;

  return (
    <TripButton
      variant="danger"
      size={size}
      className={`shadow-lg shadow-rose-900/30 ring-2 ring-white/20 ${className}`}
      onClick={() => window.open(status.downloadUrl!, "_blank", "noopener,noreferrer")}
      title={status.fileName ? `Ouvrir ${status.fileName}` : "Carte d’assistance"}
    >
      🆘 Assistance
    </TripButton>
  );
}
