import { NextResponse } from "next/server";
import { getDb } from "@/db/index";
import { preinscription } from "@/db/schema";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { recordEleveAccessAudit } from "@/app/lib/eleve-dossier-access";

/** Soumission publique de préinscription (formulaire familles). */
export async function POST(req: Request) {
  try {
    const body = (await req.json()) as {
      nom?: string;
      prenom?: string;
      dateNaissance?: string;
      lieuNaissance?: string;
      siteId?: string;
      niveauVise?: string;
      filiereVisee?: string;
      demiPension?: boolean;
      etablissementPrecedent?: string;
    };
    const nom = String(body.nom || "").trim();
    const prenom = String(body.prenom || "").trim();
    if (!nom || !prenom) {
      return NextResponse.json({ error: "Nom et prénom requis." }, { status: 400 });
    }
    const etabId = await resolveCurrentEtablissementId();
    if (!etabId) {
      return NextResponse.json(
        { error: "Établissement introuvable (tenant)." },
        { status: 400 },
      );
    }
    const db = getDb();
    const [row] = await db
      .insert(preinscription)
      .values({
        etablissementId: etabId,
        siteId: body.siteId?.trim() || null,
        niveauVise: body.niveauVise?.trim() || null,
        filiereVisee: body.filiereVisee?.trim() || null,
        nom,
        prenom,
        dateNaissance: body.dateNaissance || null,
        lieuNaissance: body.lieuNaissance?.trim() || null,
        demiPension: Boolean(body.demiPension),
        etablissementPrecedent: body.etablissementPrecedent?.trim() || null,
        status: "pending",
        payload: body,
      })
      .returning();

    await recordEleveAccessAudit({
      etablissementId: etabId,
      actorUserId: null,
      resourceType: "preinscription",
      resourceId: row.id,
      action: "create",
      metadata: { channel: "public" },
    });

    return NextResponse.json({ success: true, id: row.id });
  } catch (error) {
    console.error("[public/preinscriptions]", error);
    return NextResponse.json({ error: "Erreur serveur." }, { status: 500 });
  }
}
