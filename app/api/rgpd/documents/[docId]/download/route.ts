import { NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canAccessRgpdModule } from "@/app/lib/rgpd-access";
import { getRgpdCatalogEntry } from "@/app/lib/rgpd-catalog";
import { getRgpdProcessingFiche } from "@/app/lib/rgpd-processing-fiches";
import { loadRgpdWorkspace } from "@/app/lib/rgpd-storage";
import { getRgpdDocumentBytes } from "@/app/lib/rgpd-storage";

export async function GET(
  req: Request,
  ctx: { params: Promise<{ docId: string }> },
) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canAccessRgpdModule(roles)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const { docId } = await ctx.params;
  if (!getRgpdCatalogEntry(docId) && !getRgpdProcessingFiche(docId)) {
    return NextResponse.json({ error: "Document inconnu." }, { status: 404 });
  }

  const workspace = await loadRgpdWorkspace();
  const state = workspace.documents[docId];
  if (!state?.fileKey) {
    return NextResponse.json({ error: "Aucun fichier enregistré." }, { status: 404 });
  }

  const bytes = await getRgpdDocumentBytes(state.fileKey);
  if (!bytes) {
    return NextResponse.json({ error: "Fichier introuvable." }, { status: 404 });
  }

  const inline = new URL(req.url).searchParams.get("inline") === "1";
  const fileName = state.fileName || `${docId}.pdf`;
  const disposition = inline
    ? `inline; filename="${fileName}"`
    : `attachment; filename="${fileName}"`;

  return new NextResponse(Buffer.from(bytes), {
    headers: {
      "Content-Type": "application/pdf",
      "Content-Disposition": disposition,
    },
  });
}
