import "server-only";

import { and, eq, isNotNull } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, travelParticipant } from "@/db/schema";
import { listSanteExtraitsPourPortee } from "@/app/lib/sante-extraits-db";

export type TravelVoyageExtrait = {
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  libelle: string;
};

/**
 * Extraits santé portés sur le voyage (`portee=voyage`) pour les participants liés.
 * Lecture seule — pas le dossier médical complet.
 */
export async function listVoyageSanteExtraitsForTravel(
  etablissementId: string,
  travelId: string,
): Promise<TravelVoyageExtrait[]> {
  const db = getDb();
  const participants = await db
    .select({
      eleveId: travelParticipant.eleveId,
      nom: travelParticipant.nom,
      prenom: travelParticipant.prenom,
      classe: travelParticipant.classe,
      liveNom: eleve.nom,
      livePrenom: eleve.prenom,
      liveClasse: eleve.classe,
    })
    .from(travelParticipant)
    .leftJoin(
      eleve,
      and(eq(eleve.id, travelParticipant.eleveId), eq(eleve.etablissementId, etablissementId)),
    )
    .where(
      and(
        eq(travelParticipant.etablissementId, etablissementId),
        eq(travelParticipant.travelId, travelId),
        isNotNull(travelParticipant.eleveId),
      ),
    );

  const eleveIds = [
    ...new Set(
      participants
        .map((p) => p.eleveId)
        .filter((id): id is string => typeof id === "string" && id.length > 0),
    ),
  ];
  if (eleveIds.length === 0) return [];

  const extraits = await listSanteExtraitsPourPortee(etablissementId, "voyage", eleveIds);
  if (extraits.length === 0) return [];

  const byId = new Map(
    participants.map((p) => [
      p.eleveId!,
      {
        nom: p.liveNom || p.nom,
        prenom: p.livePrenom || p.prenom,
        classe: p.liveClasse ?? p.classe,
      },
    ]),
  );

  return extraits.map((ex) => {
    const meta = byId.get(ex.eleveId);
    return {
      eleveId: ex.eleveId,
      eleveNom: meta?.nom ?? "",
      elevePrenom: meta?.prenom ?? "",
      eleveClasse: meta?.classe ?? null,
      libelle: ex.libelle,
    };
  });
}
