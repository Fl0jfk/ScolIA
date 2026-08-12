/**
 * Surcouche Teams (messagerie interne) — masquée tant que le consentement Entra
 * / licences A1-A3 ne sont pas en place.
 *
 * Activer plus tard :
 *   TEAMS_CHAT_OVERLAY_ENABLED=true
 * ou (build) :
 *   NEXT_PUBLIC_TEAMS_CHAT_OVERLAY=1
 */
export function isTeamsChatOverlayEnabled(): boolean {
  const runtime = process.env.TEAMS_CHAT_OVERLAY_ENABLED?.trim().toLowerCase() ?? "";
  const pub = process.env.NEXT_PUBLIC_TEAMS_CHAT_OVERLAY?.trim().toLowerCase() ?? "";
  const v = runtime || pub;
  return v === "1" || v === "true" || v === "yes";
}
