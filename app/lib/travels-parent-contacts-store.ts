import "server-only";

import { and, asc, eq, inArray } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, eleveFoyerLink, foyer, foyerResponsable, foyerResponsableLien } from "@/db/schema";
import { chunkArray } from "@/app/lib/db-in-chunks";
import { recordEleveAccessAudit } from "@/app/lib/eleve-dossier-access";
import { isMissingRelationError } from "@/app/lib/foyers-import/lien-parente";
import {
  buildEleveContacts,
  tripParticipantsRaw,
  type EleveContacts,
  type FoyerContactInput,
} from "@/app/lib/travels-parent-contacts";
import type { TravelsTrip } from "@/app/lib/travels-types";

export type TripParticipantRef = { eleveId: string; nom: string; prenom: string; classe: string | null };

/**
 * Participants du voyage résolus en élèves de CET établissement (par id, sinon par INE).
 * Toutes les requêtes sont filtrées par `etabId`.
 */
export async function loadTripParticipants(trip: Pick<TravelsTrip, "data">, etabId: string): Promise<TripParticipantRef[]> {
  const raw = tripParticipantsRaw(trip);
  const ids = [...new Set(raw.map((p) => String(p.eleveId ?? "").trim()).filter(Boolean))];
  const ines = [...new Set(raw.filter((p) => !p.eleveId).map((p) => String(p.ine ?? "").trim()).filter(Boolean))];
  const db = getDb();
  const out = new Map<string, TripParticipantRef>();
  const cols = { id: eleve.id, nom: eleve.nom, prenom: eleve.prenom, classe: eleve.classe };
  for (const chunk of chunkArray(ids)) {
    const rows = await db.select(cols).from(eleve).where(and(eq(eleve.etablissementId, etabId), inArray(eleve.id, chunk)));
    for (const r of rows) out.set(r.id, { eleveId: r.id, nom: r.nom, prenom: r.prenom, classe: r.classe });
  }
  for (const chunk of chunkArray(ines)) {
    const rows = await db.select(cols).from(eleve).where(and(eq(eleve.etablissementId, etabId), inArray(eleve.ine, chunk)));
    for (const r of rows) out.set(r.id, { eleveId: r.id, nom: r.nom, prenom: r.prenom, classe: r.classe });
  }
  return [...out.values()].sort((a, b) => a.nom.localeCompare(b.nom, "fr") || a.prenom.localeCompare(b.prenom, "fr"));
}

/** Contacts (foyers nettoyés, repli fiche élève) de élèves DÉJÀ validés comme participants. */
export async function loadContactsForEleves(etabId: string, participants: TripParticipantRef[]): Promise<EleveContacts[]> {
  if (participants.length === 0) return [];
  const db = getDb();
  const eleveIds = participants.map((p) => p.eleveId);

  const fiches = new Map<string, Record<string, string | null>>();
  const links: Array<{ eleveId: string; foyerId: string }> = [];
  for (const chunk of chunkArray(eleveIds)) {
    const fr = await db
      .select({
        id: eleve.id,
        parentPhone: eleve.parentPhone,
        parent1Phone: eleve.parent1Phone,
        parent2Phone: eleve.parent2Phone,
        parentEmail: eleve.parentEmail,
        parent1Email: eleve.parent1Email,
        parent2Email: eleve.parent2Email,
      })
      .from(eleve)
      .where(and(eq(eleve.etablissementId, etabId), inArray(eleve.id, chunk)));
    for (const r of fr) fiches.set(r.id, r);
    const lr = await db
      .select({ eleveId: eleveFoyerLink.eleveId, foyerId: eleveFoyerLink.foyerId })
      .from(eleveFoyerLink)
      .where(and(eq(eleveFoyerLink.etablissementId, etabId), inArray(eleveFoyerLink.eleveId, chunk)));
    links.push(...lr);
  }

  const foyerIds = [...new Set(links.map((l) => l.foyerId))];
  const foyerRows = new Map<string, string>();
  const respRows: Array<typeof foyerResponsable.$inferSelect> = [];
  for (const chunk of chunkArray(foyerIds)) {
    const f = await db.select({ id: foyer.id, label: foyer.label }).from(foyer).where(and(eq(foyer.etablissementId, etabId), inArray(foyer.id, chunk)));
    for (const x of f) foyerRows.set(x.id, x.label);
    const r = await db
      .select()
      .from(foyerResponsable)
      .where(and(eq(foyerResponsable.etablissementId, etabId), inArray(foyerResponsable.foyerId, chunk)))
      .orderBy(asc(foyerResponsable.rang));
    respRows.push(...r);
  }

  // Liens de parenté par (responsable, élève). Table absente tant que la migration 0074 n'est pas appliquée.
  const lienByPair = new Map<string, string>();
  try {
    for (const chunk of chunkArray(eleveIds)) {
      const rows = await db
        .select({ r: foyerResponsableLien.responsableId, e: foyerResponsableLien.eleveId, lien: foyerResponsableLien.lien })
        .from(foyerResponsableLien)
        .where(and(eq(foyerResponsableLien.etablissementId, etabId), inArray(foyerResponsableLien.eleveId, chunk)));
      for (const x of rows) lienByPair.set(`${x.r}|${x.e}`, x.lien);
    }
  } catch (e) {
    if (!isMissingRelationError(e)) console.warn("[travels/parent-contacts] liens de parenté indisponibles");
  }

  const respByFoyer = new Map<string, typeof respRows>();
  for (const r of respRows) respByFoyer.set(r.foyerId, [...(respByFoyer.get(r.foyerId) ?? []), r]);

  return participants.map((p) => {
    const foyers: FoyerContactInput[] = links
      .filter((l) => l.eleveId === p.eleveId && foyerRows.has(l.foyerId))
      .map((l) => ({
        foyerId: l.foyerId,
        label: foyerRows.get(l.foyerId) ?? "Foyer",
        responsables: (respByFoyer.get(l.foyerId) ?? []).map((r) => ({
          id: r.id,
          prenom: r.prenom,
          nom: r.nom,
          email: r.email,
          telephone: r.telephone,
          autoriteParentale: r.autoriteParentale,
          contactUrgence: r.contactUrgence,
          rang: r.rang,
          lien: lienByPair.get(`${r.id}|${p.eleveId}`) ?? null,
        })),
      }));
    return buildEleveContacts({ eleve: { id: p.eleveId, nom: p.nom, prenom: p.prenom, classe: p.classe }, foyers, fiche: fiches.get(p.eleveId) ?? {} });
  });
}

/** Journal d'accès : qui, quel voyage, quel élève (ou « tous »), quand. Aucun numéro dans les métadonnées. */
export async function auditParentContacts(input: {
  etabId: string;
  actorUserId: string | null;
  tripId: string;
  eleveId?: string | null;
  action: "travel_parent_contacts_view" | "travel_parent_contacts_pdf";
  count?: number;
}): Promise<void> {
  await recordEleveAccessAudit({
    etablissementId: input.etabId,
    actorUserId: input.actorUserId,
    resourceType: "travel_parent_contacts",
    resourceId: input.tripId,
    eleveId: input.eleveId ?? null,
    action: input.action,
    metadata: input.action === "travel_parent_contacts_pdf" ? { scope: "tous participants", count: input.count ?? 0 } : { scope: "eleve" },
  });
}
