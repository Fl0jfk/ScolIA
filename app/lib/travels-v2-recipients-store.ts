import "server-only";

import { randomUUID } from "node:crypto";
import { and, eq, inArray } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { eleve, eleveFoyerLink, foyerResponsable } from "@/db/schema";
import { chunkArray } from "@/app/lib/db-in-chunks";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { listJsonRecordsInDir } from "@/app/lib/ent-json-postgres";
import { collectEleveParentEmails } from "@/app/lib/eleves-parent-emails";
import { loadElevesActifsRegistry } from "@/app/lib/eleves-registry";
import { deleteJson, putJson } from "@/app/lib/s3-storage";
import {
  mergeRecipients,
  studentsWithoutRecipient,
  type ExtraRecipient,
  type FoyerResponsableInput,
  type Recipient,
} from "@/app/lib/travels-v2-recipients";
import type { TravelsTrip } from "@/app/lib/travels-types";

const TRIP_ID_RE = /^[A-Za-z0-9_-]{8,64}$/;

export const keyOfIne = (ine: string) => String(ine || "").trim().toUpperCase();

/* ─── Adresses ajoutées pour le séjour (un enregistrement par adresse, dans le dossier du voyage) ─── */

function extrasDir(tripId: string): string {
  return `travels-v2/${tripId}/extra-recipients`;
}

export async function listExtraRecipients(tripId: string): Promise<ExtraRecipient[]> {
  if (!TRIP_ID_RE.test(tripId)) return [];
  const rows = await listJsonRecordsInDir<Record<string, unknown>>(extrasDir(tripId));
  return rows
    .filter((r) => r && r.tripId === tripId && typeof r.id === "string" && typeof r.email === "string" && typeof r.ine === "string")
    .map((r) => r as unknown as ExtraRecipient);
}

export async function addExtraRecipient(rec: Omit<ExtraRecipient, "id" | "addedAt">): Promise<ExtraRecipient> {
  const full: ExtraRecipient = { ...rec, id: randomUUID(), addedAt: new Date().toISOString() };
  await putJson(`${extrasDir(rec.tripId)}/${full.id}.json`, full);
  return full;
}

export async function removeExtraRecipient(tripId: string, id: string): Promise<void> {
  if (!TRIP_ID_RE.test(tripId) || !TRIP_ID_RE.test(id)) return;
  await deleteJson(`${extrasDir(tripId)}/${id}.json`);
}

/* ─── Lecture des foyers (lecture seule, établissement courant) ─── */

/** Responsables des foyers liés aux élèves, par INE normalisé. Sans base configurée : aucun (la fiche reste la source). */
export async function loadFoyerResponsablesByIne(ines: string[]): Promise<Map<string, FoyerResponsableInput[]>> {
  const out = new Map<string, FoyerResponsableInput[]>();
  const wanted = [...new Set(ines.map((i) => String(i || "").trim()).filter(Boolean))];
  if (wanted.length === 0 || !isDatabaseConfigured()) return out;
  try {
    const etabId = await resolveCurrentEtablissementId();
    if (!etabId) return out;
    const db = getDb();
    for (const chunk of chunkArray(wanted)) {
      const rows = await db
        .select({
          ine: eleve.ine,
          foyerId: foyerResponsable.foyerId,
          nom: foyerResponsable.nom,
          prenom: foyerResponsable.prenom,
          email: foyerResponsable.email,
          autoriteParentale: foyerResponsable.autoriteParentale,
          rang: foyerResponsable.rang,
        })
        .from(eleve)
        .innerJoin(
          eleveFoyerLink,
          and(eq(eleveFoyerLink.eleveId, eleve.id), eq(eleveFoyerLink.etablissementId, etabId)),
        )
        .innerJoin(
          foyerResponsable,
          and(eq(foyerResponsable.foyerId, eleveFoyerLink.foyerId), eq(foyerResponsable.etablissementId, etabId)),
        )
        .where(and(eq(eleve.etablissementId, etabId), inArray(eleve.ine, chunk)));
      for (const r of rows) {
        if (!r.ine) continue;
        const k = keyOfIne(r.ine);
        out.set(k, [
          ...(out.get(k) || []),
          { foyerId: r.foyerId, nom: r.nom, prenom: r.prenom, email: r.email, autoriteParentale: r.autoriteParentale, rang: r.rang },
        ]);
      }
    }
  } catch (e) {
    console.error("[voyages-v2] lecture des foyers impossible, fiches élèves seules :", e instanceof Error ? e.message : e);
  }
  return out;
}

/* ─── Résolution pour un voyage ─── */

export type TripRecipients = {
  byIne: Map<string, Recipient[]>;
  /** Élèves de la pré-liste (INE dédoublonnés), avec fiche si trouvée. */
  students: { ine: string; name: string; classe?: string; fiche: ReturnType<typeof ficheOf> | null }[];
  withoutAddress: { ine: string; name: string }[];
};

function ficheOf(e: { prenom: string; nom: string; classe?: string }) {
  return { prenom: e.prenom, nom: e.nom, classe: e.classe };
}

function displayName(prenom: string, nom: string): string {
  return `${String(prenom || "").trim()} ${String(nom || "").trim().toUpperCase()}`.trim();
}

/** Destinataires de tous les élèves de la pré-liste : fiche + foyers + adresses ajoutées pour ce séjour. */
export async function resolveTripRecipients(trip: Pick<TravelsTrip, "id" | "data">): Promise<TripRecipients> {
  const preListe = trip.data?.preListeEleves || [];
  const [eleves, extras] = await Promise.all([
    loadElevesActifsRegistry().catch(() => []),
    listExtraRecipients(trip.id).catch(() => []),
  ]);
  const ficheByIne = new Map(eleves.map((e) => [keyOfIne(e.ine || ""), e]));
  const foyers = await loadFoyerResponsablesByIne(preListe.map((p) => String(p.ine || "")));
  const extraByIne = new Map<string, ExtraRecipient[]>();
  for (const x of extras) extraByIne.set(keyOfIne(x.ine), [...(extraByIne.get(keyOfIne(x.ine)) || []), x]);

  const byIne = new Map<string, Recipient[]>();
  const students: TripRecipients["students"] = [];
  const seen = new Set<string>();
  for (const p of preListe) {
    const ine = String(p.ine || "").trim();
    const k = keyOfIne(ine);
    if (!k || seen.has(k)) continue;
    seen.add(k);
    const fiche = ficheByIne.get(k);
    byIne.set(
      k,
      mergeRecipients({
        ficheEmails: fiche ? collectEleveParentEmails(fiche) : [],
        foyerResponsables: foyers.get(k),
        extras: extraByIne.get(k),
      }),
    );
    students.push({
      ine,
      name: displayName(fiche?.prenom ?? p.prenom, fiche?.nom ?? p.nom),
      classe: fiche?.classe || p.classe || undefined,
      fiche: fiche ? ficheOf(fiche) : null,
    });
  }
  const withoutAddress = studentsWithoutRecipient(students, byIne).map((s) => ({ ine: s.ine, name: s.name }));
  return { byIne, students, withoutAddress };
}
