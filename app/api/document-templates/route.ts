import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import {
  DOCUMENT_OUTPUT_FORMATS,
  DOCUMENT_PLACEHOLDERS,
  DOCUMENT_TEMPLATES,
  INSCRIPTION_LEVELS,
  defaultAnneeScolaire,
  loadInscriptionTenantSettings,
} from "@/app/lib/document-templates";
import { getSchoolLetterhead } from "@/app/lib/pdf-branding";

export const runtime = "nodejs";

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  const [letterhead, inscriptionSettings] = await Promise.all([
    getSchoolLetterhead(),
    loadInscriptionTenantSettings(),
  ]);
  return NextResponse.json({
    templates: DOCUMENT_TEMPLATES,
    formats: DOCUMENT_OUTPUT_FORMATS,
    placeholders: DOCUMENT_PLACEHOLDERS,
    inscriptionLevels: INSCRIPTION_LEVELS.map((l) => ({
      id: l.id,
      label: l.label,
      cycle: l.cycle,
      hasOverride: Boolean(inscriptionSettings.overrides[l.id]),
    })),
    inscriptionSettings: {
      establishmentName: inscriptionSettings.establishmentName,
      accentColor: inscriptionSettings.accentColor,
    },
    defaults: {
      anneeScolaire: defaultAnneeScolaire(),
      dateDocument: new Date().toISOString().slice(0, 10),
      ville:
        letterhead.cityLine.split(/\s+/).slice(1).join(" ") ||
        letterhead.cityLine ||
        "",
    },
    branding: {
      name: letterhead.name,
      addressLine: letterhead.addressLine,
      phone: letterhead.phone,
    },
  });
}
