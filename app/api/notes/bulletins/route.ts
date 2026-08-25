import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { listGroupes } from "@/app/lib/groupes-pedagogiques-db";
import { listPeriodes } from "@/app/lib/notes-config-db";
import {
  listEleveIdsForBulletinClasse,
  listEleveIdsForBulletinGroupe,
  loadBulletinSnapshot,
} from "@/app/lib/notes-bulletins-db";

export async function POST(req: Request) {
  const gate = await requireModule("notes");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const body = await req.json().catch(() => ({}));
  const classe = String(body.classe || "").trim();
  const groupeId = String(body.groupeId || "").trim();
  const periodeId = String(body.periodeId || "").trim();

  if ((!classe && !groupeId) || !periodeId) {
    return NextResponse.json({ error: "Classe ou groupe, et période requis." }, { status: 400 });
  }

  const [eleves, periodes, groupes] = await Promise.all([
    groupeId
      ? listEleveIdsForBulletinGroupe(etabId, groupeId)
      : listEleveIdsForBulletinClasse(etabId, classe),
    listPeriodes(etabId),
    listGroupes(etabId),
  ]);
  const periode = periodes.find((p) => p.id === periodeId);
  const groupe = groupes.find((g) => g.id === groupeId);

  const previews: Array<{
    eleveId: string;
    nom: string;
    prenom: string;
    nbMatieres: number;
    nbCompetences: number;
  }> = [];
  for (const e of eleves) {
    const snapshot = await loadBulletinSnapshot(etabId, e.id, periodeId);
    previews.push({
      eleveId: e.id,
      nom: e.nom,
      prenom: e.prenom,
      nbMatieres: snapshot?.lignes.length ?? 0,
      nbCompetences: snapshot?.competences.length ?? 0,
    });
  }

  const zipQs = new URLSearchParams({ mode: "classe", periodeId });
  if (groupeId) {
    zipQs.set("groupeId", groupeId);
    if (groupe?.code) zipQs.set("classe", groupe.code);
  } else {
    zipQs.set("classe", classe);
  }

  return NextResponse.json({
    classe: groupeId ? groupe?.code || null : classe,
    groupeId: groupeId || null,
    periode,
    groupes,
    eleves: previews,
    zipUrl: `/api/notes/bulletins/pdf?${zipQs.toString()}`,
  });
}
