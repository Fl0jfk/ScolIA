/**
 * Contexte requête pour journaux d’accès (distinct du rate-limit `clientIpFromRequest`).
 * Scaleway Serverless / Knative : IP client = **première** entrée X-Forwarded-For.
 */

import { isIP } from "node:net";

const FORWARDED_FOR_MAX_LEN = 512;
const IP_FIELD_MAX_LEN = 45;

export type AuditRequestContext = {
  /** IP client validée (première entrée XFF ou X-Real-IP), sinon null. */
  clientIp: string | null;
  /** Chaîne X-Forwarded-For brute (tronquée). */
  forwardedFor: string | null;
  /** En-tête X-Envoy-External-Address si présent (tronqué). */
  envoyExternalAddress: string | null;
};

function boundedIpCandidate(raw: string | null | undefined): string | null {
  const s = raw?.trim();
  if (!s || s.length > IP_FIELD_MAX_LEN) return null;
  return isIP(s) ? s : null;
}

/** Première entrée X-Forwarded-For, validée IPv4/IPv6. */
export function validatedClientIpFromForwardedFor(
  forwardedForHeader: string | null | undefined,
): string | null {
  const first = forwardedForHeader?.split(",")[0]?.trim();
  return boundedIpCandidate(first);
}

export function auditRequestContextFromRequest(req: Request): AuditRequestContext {
  const rawXff = req.headers.get("x-forwarded-for");
  const forwardedFor = rawXff?.trim()
    ? rawXff.trim().slice(0, FORWARDED_FOR_MAX_LEN)
    : null;
  let clientIp = validatedClientIpFromForwardedFor(rawXff);
  if (!clientIp) {
    clientIp = boundedIpCandidate(req.headers.get("x-real-ip"));
  }
  const envoyRaw = req.headers.get("x-envoy-external-address")?.trim();
  const envoyExternalAddress = envoyRaw
    ? envoyRaw.slice(0, FORWARDED_FOR_MAX_LEN)
    : null;
  return {
    clientIp,
    forwardedFor,
    envoyExternalAddress,
  };
}
