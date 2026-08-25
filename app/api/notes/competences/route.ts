import { NextResponse } from "next/server";
import { requireAdmin, requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { listPeriodes } from "@/app/lib/notes-config-db";
import {
  COMPETENCE_NIVEAUX,
  buildLsuExportRows,
  listCompetenceDomaines,
  listCompetenceItems,
  listCompetenceValeurs,
  seedCompetencesCollegeDefaults,
  upsertCompetenceDomaine,
  upsertCompetenceItem,
  upsertCompetenceValeur,
} from "@/app/lib/notes-competences-db";
import { listElevesForClasse, listElevesForGroupe } from "@/app/lib/notes-saisie-db";
import { listGroupes } from "@/app/lib/groupes-pedagogiques-db";

export async function GET(req: Request) {
  const gate = await requireModule("notes");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const domaineId = url.searchParams.get("domaineId")?.trim() || "";
  const periodeId = url.searchParams.get("periodeId")?.trim() || "";
  const classe = url.searchParams.get("classe")?.trim() || "";
  const groupeId = url.searchParams.get("groupeId")?.trim() || "";
  const exportLsu = url.searchParams.get("export") === "lsu";

  if (exportLsu) {
    if ((!classe && !groupeId) || !periodeId) {
      return NextResponse.json(
        { error: "Classe ou groupe, et période requis pour l'export LSU." },
        { status: 400 },
      );
    }
    const rows = await buildLsuExportRows(etabId, {
      classe: classe || undefined,
      groupeId: groupeId || undefined,
      periodeId,
    });
    return NextResponse.json({ rows, count: rows.length });
  }

  const [domaines, periodes, groupes] = await Promise.all([
    listCompetenceDomaines(etabId),
    listPeriodes(etabId),
    listGroupes(etabId),
  ]);
  const items = domaineId ? await listCompetenceItems(etabId, domaineId) : [];
  const eleves = groupeId
    ? await listElevesForGroupe(etabId, groupeId)
    : classe
      ? await listElevesForClasse(etabId, classe)
      : [];
  const scopeReady = Boolean((classe || groupeId) && periodeId && domaineId);
  const valeurs = scopeReady
    ? await listCompetenceValeurs(etabId, {
        domaineId,
        periodeId,
        classe: classe || undefined,
        groupeId: groupeId || undefined,
      })
    : [];

  return NextResponse.json({
    domaines,
    items,
    periodes,
    groupes,
    eleves,
    valeurs,
    niveaux: COMPETENCE_NIVEAUX,
  });
}

export async function POST(req: Request) {
  const body = await req.json().catch(() => ({}));
  const action = String(body.action || "");

  if (action === "seedDefaults") {
    const adminGate = await requireAdmin();
    if (!adminGate.ok) return adminGate.response;
  } else {
    const gate = await requireModule("notes");
    if (!gate.ok) return gate.response;
  }

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  try {
    if (action === "seedDefaults") {
      const result = await seedCompetencesCollegeDefaults(etabId);
      return NextResponse.json({ ok: true, ...result });
    }
    if (action === "upsertDomaine") {
      const adminGate = await requireAdmin();
      if (!adminGate.ok) return adminGate.response;
      const row = await upsertCompetenceDomaine(etabId, body);
      return NextResponse.json({ ok: true, domaine: row });
    }
    if (action === "upsertItem") {
      const adminGate = await requireAdmin();
      if (!adminGate.ok) return adminGate.response;
      const row = await upsertCompetenceItem(etabId, body);
      return NextResponse.json({ ok: true, item: row });
    }
    if (action === "upsertValeur") {
      await upsertCompetenceValeur(etabId, {
        itemId: String(body.itemId || ""),
        eleveId: String(body.eleveId || ""),
        periodeId: String(body.periodeId || ""),
        niveau: body.niveau != null ? String(body.niveau) : null,
        appreciation: body.appreciation != null ? String(body.appreciation) : null,
      });
      return NextResponse.json({ ok: true });
    }
    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur compétences." },
      { status: 400 },
    );
  }
}
