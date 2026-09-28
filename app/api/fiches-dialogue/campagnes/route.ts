import { NextResponse } from "next/server";
import { z } from "zod";
import { requireAppUser } from "@/app/lib/app-session";
import {
  canManageFichesDialogue,
  canViewFichesDialogue,
} from "@/app/lib/fiches-dialogue-access";
import {
  FD_CAMPAGNE_TEMPLATES,
  FD_NIVEAUX,
  FD_NIVEAU_LABELS,
  presetForNiveau,
  isFdNiveau,
} from "@/app/lib/fiches-dialogue-templates";
import {
  createFdCampagneFromTemplate,
  listFdCampagnes,
} from "@/app/lib/fiches-dialogue-workflow";
import { requireTenantId } from "@/app/lib/tenant-scope";

const CatalogueSchema = z.object({
  destinations: z.array(
    z.object({
      id: z.string(),
      label: z.string(),
      niveauCible: z.string().optional(),
      interne: z.boolean().optional(),
    }),
  ),
  options: z.array(
    z.object({
      id: z.string(),
      label: z.string(),
      kind: z.enum(["lv", "option_interne", "specialite", "autre"]),
    }),
  ),
  fields: z.array(z.record(z.string(), z.unknown())).optional(),
  voiesOuverture: z
    .array(
      z.object({
        niveauActuel: z.string(),
        destinationsIds: z.array(z.string()),
      }),
    )
    .optional(),
});

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
    niveaux: FD_NIVEAUX.map((n) => ({ key: n, label: FD_NIVEAU_LABELS[n] })),
    /** @deprecated conservé pour compat anciennes UIs */
    templates: FD_CAMPAGNE_TEMPLATES.map((t) => ({
      key: t.key,
      label: t.label,
      calendrierMode: t.calendrierMode,
      starterMode: t.starterMode,
      description: t.description,
      etapesCount: t.etapes.length,
    })),
  });
}

const CreateSchema = z
  .object({
    niveauActuel: z.enum(["6e", "5e", "4e", "3e", "2nde", "1re", "Tle"]).optional(),
    templateKey: z.string().min(1).optional(),
    label: z.string().min(2).max(200),
    anneeLabel: z.string().min(4).max(32),
    anneeScolaireId: z.string().uuid().optional().nullable(),
    siteKey: z.string().max(64).optional().nullable(),
    classesCibles: z.array(z.string()).optional(),
    eleveIdsCibles: z.array(z.string().uuid()).optional(),
    delaiFamilleJours: z.number().int().min(1).max(60).optional(),
    starterMode: z.enum(["conseil_dabord", "famille_dabord"]),
    contactPpLabel: z.string().max(120).optional().nullable(),
    catalogue: CatalogueSchema.optional(),
    etapesDates: z
      .array(
        z.object({
          ordre: z.number().int().min(1),
          opensAt: z.string().nullable().optional(),
          closesAt: z.string().nullable().optional(),
          conseilDate: z.string().nullable().optional(),
        }),
      )
      .optional(),
    appelConfig: z
      .object({
        enabled: z.boolean(),
        dateLimite: z.string().optional(),
        procedureHtml: z.string().optional(),
        documentsLabels: z.array(z.string()).optional(),
        contactPpLabel: z.string().optional(),
      })
      .optional(),
  })
  .refine((d) => Boolean(d.niveauActuel || d.templateKey), {
    message: "niveauActuel ou templateKey requis",
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
    let catalogueOverride = body.data.catalogue as
      | import("@/db/schema-fiches-dialogue").FdCatalogueChoix
      | undefined;
    if (catalogueOverride && body.data.niveauActuel && isFdNiveau(body.data.niveauActuel)) {
      const preset = presetForNiveau(body.data.niveauActuel, body.data.starterMode);
      catalogueOverride = {
        ...preset.catalogue,
        destinations: catalogueOverride.destinations.length
          ? catalogueOverride.destinations
          : preset.catalogue.destinations,
        options: catalogueOverride.options.length
          ? catalogueOverride.options
          : preset.catalogue.options,
        fields: preset.catalogue.fields,
        voiesOuverture: preset.catalogue.voiesOuverture,
      };
    }

    const result = await createFdCampagneFromTemplate({
      etablissementId: scope.ctx.etablissementId,
      templateKey: body.data.templateKey,
      niveauActuel: body.data.niveauActuel,
      label: body.data.label,
      anneeLabel: body.data.anneeLabel,
      anneeScolaireId: body.data.anneeScolaireId,
      siteKey: body.data.siteKey,
      classesCibles: body.data.classesCibles,
      eleveIdsCibles: body.data.eleveIdsCibles,
      delaiFamilleJours: body.data.delaiFamilleJours,
      starterMode: body.data.starterMode,
      contactPpLabel: body.data.contactPpLabel,
      catalogueOverride,
      appelConfig: body.data.appelConfig,
      etapesDates: body.data.etapesDates,
      createdByUserId: scope.ctx.authUserId,
    });
    return NextResponse.json(result, { status: 201 });
  } catch (e) {
    const msg = e instanceof Error ? e.message : "Erreur";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
