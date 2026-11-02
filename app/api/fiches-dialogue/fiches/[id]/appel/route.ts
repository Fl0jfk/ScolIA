import { NextResponse } from "next/server";
import { z } from "zod";
import { requireAppUser } from "@/app/lib/app-session";
import { canManageFichesDialogue } from "@/app/lib/fiches-dialogue-access";
import {
  closeFdAppel,
  getFdCampagne,
  getFdFiche,
  saveFdAppelDossier,
} from "@/app/lib/fiches-dialogue-workflow";
import { requireTenantId } from "@/app/lib/tenant-scope";

type Ctx = { params: Promise<{ id: string }> };

const DocumentSchema = z.object({
  label: z.string().min(1).max(200),
  reference: z.string().max(500).optional().nullable(),
  statut: z.enum(["attendu", "recu"]).optional(),
  receivedAt: z.string().max(40).optional().nullable(),
  notes: z.string().max(2000).optional().nullable(),
});

const PatchSchema = z.object({
  commissionLabel: z.string().max(200).optional().nullable(),
  saisineDate: z.string().max(40).optional().nullable(),
  commissionDate: z.string().max(40).optional().nullable(),
  receivedAt: z.string().max(40).optional().nullable(),
  decision: z.enum(["maintien", "reformation"]).optional().nullable(),
  motif: z.string().max(4000).optional().nullable(),
  destinationProposee: z.string().max(120).optional().nullable(),
  optionsProposees: z.array(z.string()).optional(),
  informations: z.string().max(8000).optional().nullable(),
  documents: z.array(DocumentSchema).optional(),
});

const BodySchema = z.object({
  action: z.enum(["save", "close"]).default("save"),
  patch: PatchSchema.default({}),
});

function normalizePatch(input: z.infer<typeof PatchSchema>) {
  return {
    ...input,
    commissionLabel: input.commissionLabel ?? undefined,
    saisineDate: input.saisineDate ?? undefined,
    commissionDate: input.commissionDate ?? undefined,
    receivedAt: input.receivedAt ?? undefined,
    decision: input.decision ?? undefined,
    motif: input.motif ?? undefined,
    destinationProposee: input.destinationProposee ?? undefined,
    informations: input.informations ?? undefined,
    documents: input.documents?.map((d) => ({
      ...d,
      reference: d.reference ?? undefined,
      receivedAt: d.receivedAt ?? undefined,
      notes: d.notes ?? undefined,
    })),
  };
}

export async function GET(_req: Request, ctx: Ctx) {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  if (!canManageFichesDialogue(appUser.user.roles, { orgAdmin: appUser.user.orgAdmin })) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const { id } = await ctx.params;
  const fiche = await getFdFiche(scope.ctx.etablissementId, id);
  if (!fiche) return NextResponse.json({ error: "Introuvable." }, { status: 404 });
  const campagne = await getFdCampagne(scope.ctx.etablissementId, fiche.campagneId);
  if (!campagne) return NextResponse.json({ error: "Campagne introuvable." }, { status: 404 });

  return NextResponse.json({
    fiche: {
      id: fiche.id,
      eleveNom: fiche.eleveNom,
      elevePrenom: fiche.elevePrenom,
      classeActuelle: fiche.classeActuelle,
      statut: fiche.statut,
      etapeCouranteId: fiche.etapeCouranteId,
      closedAt: fiche.closedAt,
    },
    campagne: {
      id: campagne.id,
      label: campagne.label,
      anneeLabel: campagne.anneeLabel,
      appelConfig: campagne.appelConfig,
    },
    appel: fiche.appel ?? null,
  });
}

export async function PATCH(req: Request, ctx: Ctx) {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  if (!canManageFichesDialogue(appUser.user.roles, { orgAdmin: appUser.user.orgAdmin })) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const { id } = await ctx.params;
  const body = BodySchema.safeParse(await req.json().catch(() => null));
  if (!body.success) {
    return NextResponse.json({ error: "Payload invalide." }, { status: 400 });
  }
  const patch = normalizePatch(body.data.patch);
  const result =
    body.data.action === "close"
      ? await closeFdAppel({
          etablissementId: scope.ctx.etablissementId,
          ficheId: id,
          patch,
          updatedBy: appUser.user.name,
        })
      : await saveFdAppelDossier({
          etablissementId: scope.ctx.etablissementId,
          ficheId: id,
          patch,
          updatedBy: appUser.user.name,
        });

  if (!result.ok) {
    return NextResponse.json({ error: result.error }, { status: 400 });
  }
  return NextResponse.json(result);
}
