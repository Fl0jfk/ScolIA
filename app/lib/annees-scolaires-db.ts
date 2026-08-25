import "server-only";

import { and, desc, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { anneeScolaire } from "@/db/schema";
import { currentSchoolYearLabel } from "@/app/lib/ent-core-db";

export type AnneeScolaireRow = {
  id: string;
  label: string;
  startsOn: string | null;
  endsOn: string | null;
  isCurrent: boolean;
  createdAt: Date;
  updatedAt: Date;
};

function defaultBoundsForLabel(label: string): { startsOn: string; endsOn: string } {
  const m = /^(\d{4})-(\d{4})$/.exec(label.trim());
  if (m) {
    const y0 = m[1]!;
    const y1 = m[2]!;
    return { startsOn: `${y0}-09-01`, endsOn: `${y1}-08-31` };
  }
  const y = new Date().getFullYear();
  return { startsOn: `${y}-09-01`, endsOn: `${y + 1}-08-31` };
}

export async function listAnneesScolaires(etablissementId: string): Promise<AnneeScolaireRow[]> {
  const db = getDb();
  return db
    .select()
    .from(anneeScolaire)
    .where(eq(anneeScolaire.etablissementId, etablissementId))
    .orderBy(desc(anneeScolaire.label));
}

export async function getCurrentAnneeScolaire(
  etablissementId: string,
): Promise<AnneeScolaireRow | null> {
  const db = getDb();
  const [row] = await db
    .select()
    .from(anneeScolaire)
    .where(
      and(eq(anneeScolaire.etablissementId, etablissementId), eq(anneeScolaire.isCurrent, true)),
    )
    .limit(1);
  return row ?? null;
}

export async function setCurrentAnneeScolaire(
  etablissementId: string,
  anneeId: string,
): Promise<AnneeScolaireRow> {
  const db = getDb();
  const [target] = await db
    .select()
    .from(anneeScolaire)
    .where(and(eq(anneeScolaire.etablissementId, etablissementId), eq(anneeScolaire.id, anneeId)))
    .limit(1);
  if (!target) throw new Error("Année scolaire introuvable.");

  await db
    .update(anneeScolaire)
    .set({ isCurrent: false, updatedAt: new Date() })
    .where(eq(anneeScolaire.etablissementId, etablissementId));

  const [updated] = await db
    .update(anneeScolaire)
    .set({ isCurrent: true, updatedAt: new Date() })
    .where(eq(anneeScolaire.id, anneeId))
    .returning();
  return updated;
}

export async function upsertAnneeScolaire(
  etablissementId: string,
  input: {
    label: string;
    startsOn?: string | null;
    endsOn?: string | null;
    makeCurrent?: boolean;
  },
): Promise<AnneeScolaireRow> {
  const label = input.label.trim();
  if (!/^\d{4}-\d{4}$/.test(label)) {
    throw new Error("Libellé attendu au format AAAA-AAAA (ex. 2026-2027).");
  }
  const [y0, y1] = label.split("-").map(Number);
  if (!y0 || !y1 || y1 !== y0 + 1) {
    throw new Error("Les deux années doivent se suivre (ex. 2026-2027).");
  }

  const bounds = defaultBoundsForLabel(label);
  const startsOn = input.startsOn?.trim() || bounds.startsOn;
  const endsOn = input.endsOn?.trim() || bounds.endsOn;

  const db = getDb();
  const [existing] = await db
    .select()
    .from(anneeScolaire)
    .where(and(eq(anneeScolaire.etablissementId, etablissementId), eq(anneeScolaire.label, label)))
    .limit(1);

  let row: AnneeScolaireRow;
  if (existing) {
    const [updated] = await db
      .update(anneeScolaire)
      .set({
        startsOn,
        endsOn,
        updatedAt: new Date(),
      })
      .where(eq(anneeScolaire.id, existing.id))
      .returning();
    row = updated;
  } else {
    const [created] = await db
      .insert(anneeScolaire)
      .values({
        etablissementId,
        label,
        startsOn,
        endsOn,
        isCurrent: false,
      })
      .returning();
    row = created;
  }

  if (input.makeCurrent) {
    return setCurrentAnneeScolaire(etablissementId, row.id);
  }
  return row;
}

/** Crée l’année calendaire courante si absente, sans forcer le flag is_current. */
export async function ensureSuggestedAnneeScolaire(
  etablissementId: string,
): Promise<AnneeScolaireRow> {
  const label = currentSchoolYearLabel();
  const list = await listAnneesScolaires(etablissementId);
  const found = list.find((a) => a.label === label);
  if (found) return found;
  const hasCurrent = list.some((a) => a.isCurrent);
  return upsertAnneeScolaire(etablissementId, {
    label,
    makeCurrent: !hasCurrent,
  });
}

/** Id + libellé de l’année courante (crée l’année calendaire si besoin). */
export async function resolveAnneeCouranteMeta(
  etablissementId: string,
): Promise<{ id: string; label: string }> {
  const { ensureCurrentAnneeScolaire } = await import("@/app/lib/ent-core-db");
  const id = await ensureCurrentAnneeScolaire(etablissementId);
  const current = await getCurrentAnneeScolaire(etablissementId);
  if (current) return { id: current.id, label: current.label };
  const [row] = await listAnneesScolaires(etablissementId);
  if (row && row.id === id) return { id: row.id, label: row.label };
  return { id, label: currentSchoolYearLabel() };
}
