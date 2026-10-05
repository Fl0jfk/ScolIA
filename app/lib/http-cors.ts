import type { NextResponse } from "next/server";

/**
 * Origines autorisées pour les fetch cross-site (Safari / sous-domaines).
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
    [
      "Content-Type",
      "Authorization",
      "X-Requested-With",
      "RSC",
      "Next-Router-State-Tree",
      "Next-Router-Prefetch",
      "Next-Router-Segment-Prefetch",
      "Next-Url",
    ].join(", "),
  );
  response.headers.append("Vary", "Origin");
  return response;
}
