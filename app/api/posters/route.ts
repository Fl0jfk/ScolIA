import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import {
  POSTER_FORMATS,
  POSTER_TEMPLATES,
  defaultPosterDraft,
} from "@/app/lib/posters";
import { getSchoolLetterhead } from "@/app/lib/pdf-branding";

export const runtime = "nodejs";

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  const letterhead = await getSchoolLetterhead();
  return NextResponse.json({
    templates: POSTER_TEMPLATES,
    formats: POSTER_FORMATS,
    defaults: defaultPosterDraft(),
    branding: {
      name: letterhead.name,
      addressLine: letterhead.addressLine,
    },
  });
}
