import { NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canAccessRgpdModule } from "@/app/lib/rgpd-access";
import { getRgpdCatalogEntry } from "@/app/lib/rgpd-catalog";
import { getRgpdProcessingFiche } from "@/app/lib/rgpd-processing-fiches";
import { renderRgpdDocumentPdf } from "@/app/lib/rgpd-document-pdf";
import { RGPD_DOCUMENT_DISCLAIMER } from "@/app/lib/rgpd-document-content";
import {
  getRgpdEstablishmentLabel,
  getTemplateSections,
} from "@/app/lib/rgpd-templates";
import {
  generatedDocKey,
  loadRgpdWorkspace,
  saveRgpdDocumentPdf,
  saveRgpdWorkspace,
} from "@/app/lib/rgpd-storage";

const DISCLAIMER = RGPD_DOCUMENT_DISCLAIMER;

export async function POST(
  _req: Request,
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

  try {
    const workspace = await loadRgpdWorkspace();
    const establishmentLabel = await getRgpdEstablishmentLabel();
    const { title, sections } = getTemplateSections(
      docId,
      workspace.answers,
      establishmentLabel,
    );
    const pdf = await renderRgpdDocumentPdf({
      title,
      sections,
      disclaimer: DISCLAIMER,
    });

    const version = new Date().toISOString().replace(/[:.]/g, "-");
    const key = generatedDocKey(docId, version);
    await saveRgpdDocumentPdf(key, pdf);

    workspace.documents[docId] = {
      status: "genere",
      generatedAt: new Date().toISOString(),
      fileKey: key,
      fileName: `${docId}.pdf`,
    };
    workspace.history = [
      ...(workspace.history ?? []).slice(-99),
      {
        at: new Date().toISOString(),
        by:
          [user?.firstName, user?.lastName].filter(Boolean).join(" ") || "Utilisateur",
        action: "DOCUMENT_GENERATED",
        note: entry?.title ?? fiche?.title ?? docId,
      },
    ];
    await saveRgpdWorkspace(workspace);

    return NextResponse.json({
      success: true,
      docId,
      fileName: `${docId}.pdf`,
      generatedAt: workspace.documents[docId].generatedAt,
    });
  } catch (e) {
    console.error("[rgpd/generate]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Génération impossible" },
      { status: 500 },
    );
  }
}
