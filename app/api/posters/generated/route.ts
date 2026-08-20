import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { formatLabel, loadPosterGeneratedIndex } from "@/app/lib/posters";

export const runtime = "nodejs";

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const items = await loadPosterGeneratedIndex();
  return NextResponse.json({
    items: items.map((e) => ({
      ...e,
      formatLabel: formatLabel(e.format),
      downloadUrl: `/api/posters/generated/${e.id}/pdf`,
    })),
  });
}
