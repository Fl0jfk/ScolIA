import { NextResponse } from "next/server";
import { loadAppConfig } from "@/app/lib/app-config";
import { resolveHeaderLogoObjectKey } from "@/app/lib/branding-logo";
import { getObjectBytes } from "@/app/lib/s3-storage";
import { getTenant } from "@/app/lib/tenant-context";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

function contentTypeForKey(key: string): string {
  const lower = key.toLowerCase();
  if (lower.endsWith(".png")) return "image/png";
  if (lower.endsWith(".jpg") || lower.endsWith(".jpeg")) return "image/jpeg";
  if (lower.endsWith(".webp")) return "image/webp";
  if (lower.endsWith(".svg")) return "image/svg+xml";
  if (lower.endsWith(".gif")) return "image/gif";
  return "application/octet-stream";
}

/**
 * Logo header du tenant courant — proxy same-origin vers S3.
 * Évite les 403 intermittents des URLs présignées (expire 1h + cache localStorage).
 */
export async function GET() {
  try {
    const [config, tenant] = await Promise.all([loadAppConfig(), getTenant()]);
    const rawLogo = config.identity.headerLogoUrl?.trim() || tenant.logoUrl?.trim() || "";
    const key = await resolveHeaderLogoObjectKey(rawLogo);
    if (!key) {
      return NextResponse.json({ error: "Logo introuvable." }, { status: 404 });
    }

    const bytes = await getObjectBytes(key);
    if (!bytes?.length) {
      return NextResponse.json({ error: "Logo introuvable." }, { status: 404 });
    }

    const fileName = key.split("/").pop() || "logo";
    return new NextResponse(new Uint8Array(bytes), {
      status: 200,
      headers: {
        "Content-Type": contentTypeForKey(key),
        "Content-Length": String(bytes.length),
        "Cache-Control": "public, max-age=300, stale-while-revalidate=3600",
        "Content-Disposition": `inline; filename="${fileName.replace(/"/g, "")}"`,
      },
    });
  } catch (e) {
    console.error("[site/header-logo]", e);
    return NextResponse.json({ error: "Logo indisponible." }, { status: 500 });
  }
}
