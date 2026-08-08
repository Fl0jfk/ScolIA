import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { loadGeneratedIndex } from "@/app/lib/document-templates";

export const runtime = "nodejs";

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const items = await loadGeneratedIndex();
  return NextResponse.json({
    items: items.map((e) => ({
      ...e,
      format: e.format || "pdf",
      downloadUrl: `/api/document-templates/generated/${e.id}/pdf`,
    })),
  });
}
