import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { DOCUMENT_TEMPLATES, defaultAnneeScolaire } from "@/app/lib/document-templates";
import { getSchoolLetterhead } from "@/app/lib/pdf-branding";

export const runtime = "nodejs";

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  const letterhead = await getSchoolLetterhead();
  return NextResponse.json({
    templates: DOCUMENT_TEMPLATES,
    defaults: {
      anneeScolaire: defaultAnneeScolaire(),
      dateDocument: new Date().toISOString().slice(0, 10),
      ville: letterhead.cityLine.split(/\s+/).slice(1).join(" ") || letterhead.cityLine || "",
    },
    branding: {
      name: letterhead.name,
      addressLine: letterhead.addressLine,
      phone: letterhead.phone,
    },
  });
}
