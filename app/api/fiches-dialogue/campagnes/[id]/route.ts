import { NextResponse } from "next/server";
import { z } from "zod";
import { requireAppUser } from "@/app/lib/app-session";
import {
  canManageFichesDialogue,
  canViewFichesDialogue,
} from "@/app/lib/fiches-dialogue-access";
import {
  getFdCampagne,
  getFdCampagneStats,
  listFdEtapes,
  listFdFiches,
  updateFdCampagne,
  updateFdEtapeDates,
} from "@/app/lib/fiches-dialogue-workflow";
import { requireTenantId } from "@/app/lib/tenant-scope";

type Ctx = { params: Promise<{ id: string }> };

export async function GET(_req: Request, ctx: Ctx) {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  if (!canViewFichesDialogue(appUser.user.roles, { orgAdmin: appUser.user.orgAdmin })) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const { id } = await ctx.params;
  const campagne = await getFdCampagne(scope.ctx.etablissementId, id);
  if (!campagne) return NextResponse.json({ error: "Introuvable." }, { status: 404 });
  const etapes = await listFdEtapes(scope.ctx.etablissementId, id);
  const fiches = await listFdFiches(scope.ctx.etablissementId, id);
  const stats = await getFdCampagneStats(scope.ctx.etablissementId, id);
  return NextResponse.json({ campagne, etapes, fiches, stats });
}

const PatchSchema = z.object({
  label: z.string().min(2).max(200).optional(),
  statut: z.enum(["brouillon", "active", "cloturee", "archivee"]).optional(),
  delaiFamilleJours: z.number().int().min(1).max(60).optional(),
  classesCibles: z.array(z.string()).optional(),
  calendrierMode: z.enum(["trimestre", "semestre", "personnalise"]).optional(),
  catalogue: z
    .object({
      destinations: z.array(
        z.object({ id: z.string(), label: z.string(), niveauCible: z.string().optional() }),
      ),
      options: z.array(
        z.object({
          id: z.string(),
          label: z.string(),
          kind: z.enum(["lv", "option_interne", "specialite", "autre"]),
        }),
      ),
      fields: z.array(
        z.object({
          id: z.string(),
          type: z.enum(["select", "multiselect", "text", "textarea", "checkbox"]),
          label: z.string(),
          required: z.boolean().optional(),
          optionsFrom: z.enum(["destinations", "options"]).optional(),
          inlineOptions: z.array(z.object({ id: z.string(), label: z.string() })).optional(),
          helpText: z.string().optional(),
        }),
      ),
    })
    .optional(),
  appelConfig: z
    .object({
      enabled: z.boolean(),
      dateLimite: z.string().optional(),
      procedureHtml: z.string().optional(),
      documentsLabels: z.array(z.string()).optional(),
    })
    .optional(),
  etapes: z
    .array(
      z.object({
        id: z.string().uuid(),
        label: z.string().optional(),
        description: z.string().nullable().optional(),
        opensAt: z.string().datetime().nullable().optional(),
        closesAt: z.string().datetime().nullable().optional(),
        conseilDate: z.string().nullable().optional(),
      }),
    )
    .optional(),
});

export async function PATCH(req: Request, ctx: Ctx) {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  if (!canManageFichesDialogue(appUser.user.roles, { orgAdmin: appUser.user.orgAdmin })) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const { id } = await ctx.params;
  const body = PatchSchema.safeParse(await req.json().catch(() => null));
  if (!body.success) {
    return NextResponse.json({ error: "Payload invalide." }, { status: 400 });
  }

  try {
    const campagne = await updateFdCampagne(scope.ctx.etablissementId, id, {
      label: body.data.label,
      statut: body.data.statut,
      delaiFamilleJours: body.data.delaiFamilleJours,
      classesCibles: body.data.classesCibles,
      calendrierMode: body.data.calendrierMode,
      catalogue: body.data.catalogue,
      appelConfig: body.data.appelConfig,
    });

    if (body.data.etapes?.length) {
      for (const e of body.data.etapes) {
        await updateFdEtapeDates(scope.ctx.etablissementId, e.id, {
          label: e.label,
          description: e.description,
          opensAt: e.opensAt === undefined ? undefined : e.opensAt ? new Date(e.opensAt) : null,
          closesAt: e.closesAt === undefined ? undefined : e.closesAt ? new Date(e.closesAt) : null,
          conseilDate: e.conseilDate === undefined ? undefined : e.conseilDate,
        });
      }
    }

    const etapes = await listFdEtapes(scope.ctx.etablissementId, id);
    return NextResponse.json({ campagne, etapes });
  } catch (e) {
    const msg = e instanceof Error ? e.message : "Erreur";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
