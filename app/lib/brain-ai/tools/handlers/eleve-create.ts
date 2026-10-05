import "server-only";

import { choicesResult } from "@/app/lib/brain-ai/choice-options";
import type { BrainClientAction, BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";
import { canManageElevePreinscriptions } from "@/app/lib/eleve-dossier-scope";
import { createElevePreinscrit } from "@/app/lib/eleve-create-preinscrit";
import { isValidParentEmail } from "@/app/lib/eleves-parent-emails";
import { recordEleveAccessAudit } from "@/app/lib/eleve-dossier-access";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { isDatabaseConfigured } from "@/db/index";

/**
 * Création manuelle d’un élève préinscrit (wizard + confirmation).
 */
export async function handleCreateElevePreinscrit(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) {
    return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  }
  if (
    !canManageElevePreinscriptions({
      roles: ctx.roles,
      orgAdmin: ctx.isOrgAdmin,
      platformAdmin: false,
    })
  ) {
    return {
      ok: false,
      error: "Création réservée à la direction, l’admin et l’administratif.",
      code: "FORBIDDEN",
    };
  }
  if (!isDatabaseConfigured()) {
    return { ok: false, error: "Base indisponible.", code: "DB" };
  }
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return { ok: false, error: "Établissement introuvable.", code: "DB" };

  const nom = String(args.nom || args.lastName || "").trim();
  const prenom = String(args.prenom || args.firstName || "").trim();
  const parentEmail = String(args.parentEmail || args.email || "").trim().toLowerCase();
  const parentPhone = String(args.parentPhone || args.phone || "").trim() || null;
  const parentFirstName = String(args.parentFirstName || "").trim() || null;
  const parentLastName = String(args.parentLastName || "").trim() || null;
  const classe = String(args.classe || args.className || "").trim() || null;

  if (!nom) {
    return choicesResult(
      "create_eleve_preinscrit",
      "nom",
      "Nom de famille de l’élève ?",
      [],
      {
        prenom: prenom || undefined,
        parentEmail: parentEmail || undefined,
        parentPhone,
        parentFirstName,
        parentLastName,
        classe,
      },
      "text",
    );
  }
  if (!prenom) {
    return choicesResult(
      "create_eleve_preinscrit",
      "prenom",
      `Prénom de ${nom} ?`,
      [],
      {
        nom,
        parentEmail: parentEmail || undefined,
        parentPhone,
        parentFirstName,
        parentLastName,
        classe,
      },
      "text",
    );
  }
  if (!parentEmail) {
    return choicesResult(
      "create_eleve_preinscrit",
      "parentEmail",
      `E-mail d’un parent / responsable pour ${prenom} ${nom} ?`,
      [],
      {
        nom,
        prenom,
        parentPhone,
        parentFirstName,
        parentLastName,
        classe,
      },
      "text",
    );
  }
  if (!isValidParentEmail(parentEmail)) {
    return {
      ok: false,
      error: "E-mail parent invalide.",
      code: "VALIDATION",
    };
  }

  if (!ctx.confirmed) {
    return {
      ok: false,
      needsConfirmation: true,
      tool: "create_eleve_preinscrit",
      args: {
        nom,
        prenom,
        parentEmail,
        parentPhone,
        parentFirstName,
        parentLastName,
        classe,
      },
      summaryFr: `Créer le dossier préinscrit ${prenom} ${nom}${
        classe ? ` (${classe})` : ""
      } — contact ${parentEmail}${parentPhone ? ` / ${parentPhone}` : ""}.`,
    };
  }

  try {
    const created = await createElevePreinscrit({
      etablissementId: etabId,
      nom,
      prenom,
      parents: [
        {
          firstName: parentFirstName,
          lastName: parentLastName,
          email: parentEmail,
          phone: parentPhone,
        },
      ],
      classe,
      sourcePrefix: "manuel-scolia",
    });

    await recordEleveAccessAudit({
      etablissementId: etabId,
      actorUserId: ctx.userId,
      resourceType: "eleve",
      resourceId: created.id,
      eleveId: created.id,
      action: "create",
      metadata: {
        source: "manuel-scolia",
        status: "preinscrit",
        parentCount: 1,
      },
    });

    const openAction: BrainClientAction = {
      type: "open_eleve_dossier",
      eleveId: created.id,
      subView: "inscription",
    };

    return {
      ok: true,
      data: {
        eleve: created,
        dossierUrl: `/eleves/dossier/${created.id}`,
        inscriptionDocsUrl: `/eleves/dossier/${created.id}/inscription`,
        clientActions: [openAction],
        ctas: [
          { label: "Ouvrir le dossier", href: `/eleves/dossier/${created.id}` },
          {
            label: "Docs inscription",
            href: `/eleves/dossier/${created.id}/inscription`,
          },
        ],
      },
      summaryFr: `Dossier préinscrit créé pour ${created.prenom} ${created.nom}.`,
    };
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return { ok: false, error: msg };
  }
}
