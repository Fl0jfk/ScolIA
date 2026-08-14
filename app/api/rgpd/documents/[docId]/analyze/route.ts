import { NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canAccessRgpdModule } from "@/app/lib/rgpd-access";
import { getRgpdCatalogEntry } from "@/app/lib/rgpd-catalog";
import { analyzeRgpdDocumentWithAi } from "@/app/lib/rgpd-ai";
import { computeRgpdComplianceScore } from "@/app/lib/rgpd-scoring";
import {
  getRgpdDocumentBytes,
  loadRgpdWorkspace,
  saveRgpdWorkspace,
} from "@/app/lib/rgpd-storage";
import { extractTextFromUpload } from "@/app/lib/personnel-document-text";

export async function POST(
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
  const entry = getRgpdCatalogEntry(docId);
  if (!entry || entry.incidentTool) {
    return NextResponse.json({ error: "Document inconnu." }, { status: 404 });
  }

  try {
    const body = await req.json();
    const key = String(body.key || "").trim();
    const fileName = String(body.fileName || "document.pdf");
    if (!key) {
      return NextResponse.json({ error: "Clé S3 requise." }, { status: 400 });
    }

    const bytes = await getRgpdDocumentBytes(key);
    if (!bytes) {
      return NextResponse.json({ error: "Fichier introuvable." }, { status: 404 });
    }

    const text = await extractTextFromUpload(bytes, fileName, body.contentType || "");
    const workspace = await loadRgpdWorkspace();
    const analysis = await analyzeRgpdDocumentWithAi({
      entry,
      documentText: text,
      answers: workspace.answers,
    });

    workspace.documents[docId] = {
      status: "importe",
      importedAt: new Date().toISOString(),
      fileKey: key,
      fileName,
      analysis: { ...analysis, analyzedAt: new Date().toISOString() },
    };
    workspace.history = [
      ...(workspace.history ?? []).slice(-99),
      {
        at: new Date().toISOString(),
        by:
          [user?.firstName, user?.lastName].filter(Boolean).join(" ") || "Utilisateur",
        action: "DOCUMENT_ANALYZED",
        note: `${entry.title} — score ${analysis.documentScore}/100`,
      },
    ];
    await saveRgpdWorkspace(workspace);

    return NextResponse.json({
      analysis,
      score: computeRgpdComplianceScore(workspace),
    });
  } catch (e) {
    console.error("[rgpd/analyze]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Analyse impossible" },
      { status: 500 },
    );
  }
}
