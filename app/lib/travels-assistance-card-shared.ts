/** Types / helpers partagés client + serveur (sans server-only). */

export function travelsAssistanceCardDownloadApiPath(): string {
  return "/api/travels/assistance-card?raw=1";
}

export type TravelsAssistanceCardApiStatus = {
  configured: boolean;
  fileName: string | null;
  downloadUrl: string | null;
};

export const TRAVELS_ASSISTANCE_CARD_CHANGED_EVENT = "travels-assistance-card-changed";

/** Forme JSON stable pour GET /api/travels/assistance-card (sans ?raw=1). */
export function travelsAssistanceCardApiStatusFromResolved(
  resolved: { fileName: string } | null,
): TravelsAssistanceCardApiStatus {
  if (!resolved) {
    return { configured: false, fileName: null, downloadUrl: null };
  }
  return {
    configured: true,
    fileName: resolved.fileName,
    downloadUrl: travelsAssistanceCardDownloadApiPath(),
  };
}
