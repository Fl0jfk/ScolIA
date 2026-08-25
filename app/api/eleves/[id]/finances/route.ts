import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { listUserRolesFromDb } from "@/app/lib/auth-roles-db";
import { eleveDossierSectionsForRoles } from "@/app/lib/eleve-dossier-access";
import { getAppSession } from "@/app/lib/intranet-session";
import {
  createFactureForEleveFoyer,
  emitFacture,
  generateFacturePdf,
  loadFinancesForEleve,
  noterRelanceFacture,
  solderFacture,
  upsertFoyerFacturation,
} from "@/app/lib/facturation-db";

type Ctx = { params: Promise<{ id: string }> };

async function canAccessFinances(etabId: string): Promise<boolean> {
  const session = await getAppSession();
  if (!session?.user) return false;
  const roles =
    session.user.roles.length > 0
      ? session.user.roles
      : await listUserRolesFromDb(session.user.id, etabId);
  const sections = eleveDossierSectionsForRoles(roles, {
    orgAdmin: session.user.orgAdmin,
    platformAdmin: session.user.platformAdmin,
  });
  return sections.has("facturation");
}

export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  if (!(await canAccessFinances(etabId))) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
  }

  const { id } = await ctx.params;
  const finances = await loadFinancesForEleve(etabId, id);
  return NextResponse.json({ finances });
}

export async function POST(req: Request, ctx: Ctx) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  if (!(await canAccessFinances(etabId))) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
  }

  const { id: eleveId } = await ctx.params;
  const body = await req.json().catch(() => ({}));
  const action = String(body.action || "");

  try {
    if (action === "upsertFoyerFacturation") {
      const row = await upsertFoyerFacturation(etabId, {
        foyerId: String(body.foyerId || ""),
        codeAuxiliaire: body.codeAuxiliaire,
        categorieQuotient: body.categorieQuotient,
        quotientFamilial: body.quotientFamilial,
        iban: body.iban,
        bic: body.bic,
        rum: body.rum,
        mandatDate: body.mandatDate,
        acceptePrelevement: body.acceptePrelevement,
      });
      return NextResponse.json({ ok: true, facturation: row });
    }

    if (action === "createFacture") {
      const facture = await createFactureForEleveFoyer(etabId, {
        eleveId,
        foyerId: String(body.foyerId || ""),
        autoTarifs: body.autoTarifs !== false,
      });
      return NextResponse.json({ ok: true, facture });
    }

    if (action === "emitFacture") {
      const factureId = String(body.factureId || "");
      const row = await emitFacture(etabId, factureId);
      return NextResponse.json({ ok: true, facture: row });
    }

    if (action === "generatePdf") {
      const factureId = String(body.factureId || "");
      const result = await generateFacturePdf(etabId, factureId);
      return NextResponse.json({ ok: true, ...result });
    }

    if (action === "solderFacture") {
      const result = await solderFacture(etabId, String(body.factureId || ""), {
        mode: body.mode,
        reference: body.reference,
      });
      return NextResponse.json({ ok: true, ...result });
    }

    if (action === "noterRelance") {
      const result = await noterRelanceFacture(
        etabId,
        String(body.factureId || ""),
        body.note ? String(body.note) : undefined,
      );
      return NextResponse.json({ ok: true, ...result });
    }

    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Opération impossible." },
      { status: 400 },
    );
  }
}
