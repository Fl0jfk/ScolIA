import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import {
  contentTypeForFormat,
  extensionForFormat,
  loadGeneratedDocument,
  loadGeneratedFileBytes,
} from "@/app/lib/document-templates";

export const runtime = "nodejs";

type Ctx = { params: Promise<{ id: string }> };

export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  const { id } = await ctx.params;
  const clean = String(id || "").replace(/[^\w.-]+/g, "");
  if (!clean) return NextResponse.json({ error: "id invalide" }, { status: 400 });

  const meta = await loadGeneratedDocument(clean);
  if (!meta) return NextResponse.json({ error: "Document introuvable" }, { status: 404 });

  const bytes = await loadGeneratedFileBytes(clean);
  if (!bytes?.length) {
    return NextResponse.json({ error: "Fichier introuvable" }, { status: 404 });
  }

  const format = meta.format || "pdf";
  const ext = extensionForFormat(format);
  const filename = `${meta.title || meta.id}.${ext}`.replace(/[^\w.\-àâäéèêëïîôùûüç\s]+/gi, "_");
  return new NextResponse(new Uint8Array(bytes), {
    status: 200,
    headers: {
      "Content-Type": contentTypeForFormat(format),
      "Content-Disposition": `attachment; filename="${filename}"`,
      "Cache-Control": "private, no-store",
    },
  });
}
