import type { NextResponse } from "next/server";

/**
 * Origines autorisées pour les fetch cross-site (Safari / iframe Collabora / sous-domaines).
 */
export function isAllowedWebOrigin(origin: string): boolean {
  const raw = origin.trim();
  if (!raw || raw === "null") return false;
  try {
    const url = new URL(raw);
    const host = url.hostname.toLowerCase();
    if (url.protocol !== "http:" && url.protocol !== "https:") return false;
    if (host === "localhost" || host === "127.0.0.1") return true;
    if (host === "scolia.fr" || host.endsWith(".scolia.fr")) return true;
    if (host.endsWith(".functions.fnc.fr-par.scw.cloud")) return true;
    for (const candidate of [
      process.env.COLLABORA_URL,
      process.env.NEXT_PUBLIC_COLLABORA_URL,
    ]) {
      const v = candidate?.trim();
      if (!v) continue;
      try {
        if (new URL(v).origin === url.origin) return true;
      } catch {
        /* ignore */
      }
    }
    return false;
  } catch {
    return false;
  }
}

export function applyCorsHeaders(request: Request, response: NextResponse): NextResponse {
  const origin = request.headers.get("origin") || "";
  if (!origin || !isAllowedWebOrigin(origin)) return response;
  response.headers.set("Access-Control-Allow-Origin", origin);
  response.headers.set("Access-Control-Allow-Credentials", "true");
  response.headers.set("Access-Control-Allow-Methods", "GET, POST, PUT, PATCH, DELETE, OPTIONS");
  response.headers.set(
    "Access-Control-Allow-Headers",
    "Content-Type, Authorization, Cookie, X-Requested-With",
  );
  response.headers.append("Vary", "Origin");
  return response;
}
