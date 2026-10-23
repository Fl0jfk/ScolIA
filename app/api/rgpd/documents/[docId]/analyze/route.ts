import { NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canAccessRgpdModule } from "@/app/lib/rgpd-access";
import { getRgpdCatalogEntry } from "@/app/lib/rgpd-catalog";
import { getRgpdProcessingFiche } from "@/app/lib/rgpd-processing-fiches";
import {
  analyzeRgpdDocumentWithAi,
  toRgpdAnalyzableFromCatalog,
  toRgpdAnalyzableFromFiche,
} from "@/app/lib/rgpd-ai";
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
  const fiche = getRgpdProcessingFiche(docId);
  if ((!entry || entry.incidentTool) && !fiche) {
    return NextResponse.json({ error: "Document inconnu." }, { status: 404 });
  }

  const analyzable = entry && !entry.incidentTool
    ? toRgpdAnalyzableFromCatalog(entry)
    : toRgpdAnalyzableFromFiche(fiche!);

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
    const actor =
      [user?.firstName, user?.lastName].filter(Boolean).join(" ") || "Utilisateur";

    // Toujours enregistrer l'import : l'analyse IA ne doit pas faire perdre le fichier.
    workspace.documents[docId] = {
      status: "importe",
      importedAt: new Date().toISOString(),
      fileKey: key,
      fileName,
      analysis: workspace.documents[docId]?.analysis,
    };

    try {
      const analysis = await analyzeRgpdDocumentWithAi({
        entry: analyzable,
        documentText: text,
        answers: workspace.answers,
      });

      workspace.documents[docId] = {
        ...workspace.documents[docId],
        status: "importe",
        importedAt: workspace.documents[docId].importedAt,
        fileKey: key,
        fileName,
        analysis: { ...analysis, analyzedAt: new Date().toISOString() },
      };
      workspace.history = [
        ...(workspace.history ?? []).slice(-99),
        {
          at: new Date().toISOString(),
          by: actor,
          action: "DOCUMENT_ANALYZED",
          note: `${analyzable.title} — score ${analysis.documentScore}/100`,
        },
      ];
      await saveRgpdWorkspace(workspace);

      return NextResponse.json({
        analysis,
        analysisPending: false,
        score: computeRgpdComplianceScore(workspace),
      });
    } catch (aiError) {
      console.error("[rgpd/analyze] AI", aiError);
      workspace.history = [
        ...(workspace.history ?? []).slice(-99),
        {
          at: new Date().toISOString(),
          by: actor,
          action: "DOCUMENT_IMPORTED",
          note: `${analyzable.title} — import OK, analyse IA différée`,
        },
      ];
      await saveRgpdWorkspace(workspace);

      const message =
        aiError instanceof Error
          ? aiError.message
          : "Analyse IA temporairement indisponible.";

      return NextResponse.json({
        analysis: null,
        analysisPending: true,
        warning: message,
        score: computeRgpdComplianceScore(workspace),
      });
    }
  } catch (e) {
    console.error("[rgpd/analyze]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Analyse impossible" },
      { status: 500 },
    );
  }
}
