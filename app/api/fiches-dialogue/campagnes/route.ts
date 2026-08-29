import { NextResponse } from "next/server";
import { z } from "zod";
import { requireAppUser } from "@/app/lib/app-session";
import {
  canManageFichesDialogue,
  canViewFichesDialogue,
} from "@/app/lib/fiches-dialogue-access";
import { FD_CAMPAGNE_TEMPLATES } from "@/app/lib/fiches-dialogue-templates";
import {
  createFdCampagneFromTemplate,
  listFdCampagnes,
} from "@/app/lib/fiches-dialogue-workflow";
import { requireTenantId } from "@/app/lib/tenant-scope";

export async function GET() {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }
  if (!canViewFichesDialogue(appUser.user.roles, { orgAdmin: appUser.user.orgAdmin })) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const campagnes = await listFdCampagnes(scope.ctx.etablissementId);
  return NextResponse.json({
    campagnes,
    templates: FD_CAMPAGNE_TEMPLATES.map((t) => ({
      key: t.key,
      label: t.label,
      calendrierMode: t.calendrierMode,
      description: t.description,
      etapesCount: t.etapes.length,
    })),
  });
}

const CreateSchema = z.object({
  templateKey: z.string().min(1),
  label: z.string().min(2).max(200),
  anneeLabel: z.string().min(4).max(32),
  anneeScolaireId: z.string().uuid().optional().nullable(),
  siteKey: z.string().max(64).optional().nullable(),
  classesCibles: z.array(z.string()).optional(),
  delaiFamilleJours: z.number().int().min(1).max(60).optional(),
  appelConfig: z
    .object({
      enabled: z.boolean(),
      dateLimite: z.string().optional(),
      procedureHtml: z.string().optional(),
      documentsLabels: z.array(z.string()).optional(),
    })
    .optional(),
});

export async function POST(req: Request) {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }
  if (!canManageFichesDialogue(appUser.user.roles, { orgAdmin: appUser.user.orgAdmin })) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const body = CreateSchema.safeParse(await req.json().catch(() => null));
  if (!body.success) {
    return NextResponse.json({ error: "Payload invalide.", details: body.error.flatten() }, { status: 400 });
  }

  try {
    const result = await createFdCampagneFromTemplate({
      etablissementId: scope.ctx.etablissementId,
      templateKey: body.data.templateKey,
      label: body.data.label,
      anneeLabel: body.data.anneeLabel,
      anneeScolaireId: body.data.anneeScolaireId,
      siteKey: body.data.siteKey,
      classesCibles: body.data.classesCibles,
      delaiFamilleJours: body.data.delaiFamilleJours,
      appelConfig: body.data.appelConfig,
      createdByUserId: scope.ctx.authUserId,
    });
    return NextResponse.json(result, { status: 201 });
  } catch (e) {
    const msg = e instanceof Error ? e.message : "Erreur";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
