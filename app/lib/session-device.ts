/** Types & libellés sessions — partageables client / serveur (sans server-only). */

export type SessionDevicePublic = {
  id: string;
  createdAt: string;
  updatedAt: string;
  expiresAt: string;
  ipAddress: string | null;
  userAgent: string | null;
  deviceLabel: string;
  current: boolean;
};

/** Libellé lisible navigateur / OS à partir du User-Agent. */
export function describeUserAgent(ua: string | null | undefined): string {
  const raw = String(ua || "").trim();
  if (!raw) return "Appareil inconnu";

  let browser = "Navigateur";
  if (/Edg\//i.test(raw)) browser = "Edge";
  else if (/Chrome\//i.test(raw) && !/Chromium/i.test(raw)) browser = "Chrome";
  else if (/Firefox\//i.test(raw)) browser = "Firefox";
  else if (/Safari\//i.test(raw) && !/Chrome\//i.test(raw)) browser = "Safari";
  else if (/OPR\//i.test(raw) || /Opera/i.test(raw)) browser = "Opera";

  let os = "appareil";
  if (/Windows NT/i.test(raw)) os = "Windows";
  else if (/Android/i.test(raw)) os = "Android";
  else if (/iPhone|iPad|iPod/i.test(raw)) os = "iOS";
  else if (/Mac OS X/i.test(raw)) os = "macOS";
  else if (/Linux/i.test(raw)) os = "Linux";

  const mobile = /Mobile|Android|iPhone|iPad/i.test(raw);
  const kind = mobile ? "Mobile" : "Ordinateur";
  return `${browser} · ${os} (${kind})`;
}
