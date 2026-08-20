import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import {
  POSTER_FORMATS,
  POSTER_PALETTE,
  POSTER_TEMPLATES,
  defaultPosterDraft,
} from "@/app/lib/posters";
import { resolveHeaderLogoDisplayUrl } from "@/app/lib/branding-logo";
import { loadAppConfig } from "@/app/lib/app-config";
import { getSchoolLetterhead } from "@/app/lib/pdf-branding";
import { getTenant } from "@/app/lib/tenant-context";

export const runtime = "nodejs";

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  const [letterhead, config, tenant] = await Promise.all([
    getSchoolLetterhead(),
    loadAppConfig(),
    getTenant(),
  ]);
  const rawLogo = config.identity.headerLogoUrl?.trim() || tenant.logoUrl?.trim() || "";
  const logoUrl = await resolveHeaderLogoDisplayUrl(rawLogo);

  return NextResponse.json({
    templates: POSTER_TEMPLATES,
    formats: POSTER_FORMATS,
    palette: POSTER_PALETTE,
    defaults: defaultPosterDraft(),
    branding: {
      name: letterhead.name,
      addressLine: letterhead.addressLine,
      logoUrl,
    },
  });
}
