import "server-only";

import { and, eq, or } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, vsAbsenceEleve } from "@/db/schema";
import {
  isEntCoreDbEnabled,
  resolveCurrentEtablissementId,
} from "@/app/lib/ent-core-db";
import { findEleveByIne } from "@/app/lib/eleves-registry";
import { createAbsenceAccueilEleve } from "@/app/lib/vs-absences-db";
import { getStageConvention, saveStageConvention } from "@/app/lib/stage-storage";
import type { StageConvention } from "@/app/lib/stage-types";

function motifForConvention(convention: StageConvention): string {
  const company = convention.company.name?.trim() || "entreprise";
  return `Stage — ${company} [${convention.id.slice(-10)}]`;
}

async function resolveEleveIdSoft(
  convention: StageConvention,
  etablissementId: string,
): Promise<{ eleveId: string } | null> {
  const db = getDb();
  const ine = convention.ocrMeta?.matchedEleveIne?.trim().toUpperCase() || "";
  if (ine) {
    const [byIne] = await db
      .select({ id: eleve.id })
      .from(eleve)
      .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.ine, ine)))
      .limit(1);
    if (byIne) return { eleveId: byIne.id };
    const registry = await findEleveByIne(ine);
    if (registry?.id) {
      const [byId] = await db
        .select({ id: eleve.id })
        .from(eleve)
        .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.id, registry.id)))
        .limit(1);
      if (byId) return { eleveId: byId.id };
    }
  }

  const nom = convention.student.lastName.trim();
  const prenom = convention.student.firstName.trim();
  if (!nom || !prenom) return null;
  const candidates = await db
    .select({ id: eleve.id })
    .from(eleve)
    .where(
      and(
        eq(eleve.etablissementId, etablissementId),
        or(
          and(eq(eleve.nom, nom), eq(eleve.prenom, prenom)),
          and(eq(eleve.nom, nom.toUpperCase()), eq(eleve.prenom, prenom)),
        ),
      ),
    )
    .limit(5);
  if (candidates.length === 1) return { eleveId: candidates[0]!.id };
  return null;
}

/**
 * Déclare une absence élève (source accueil) sur toute la période de stage.
 * Idempotent : ne recrée pas si déjà liée à la convention.
 */
export async function ensureStageAbsencesForConvention(
  convention: StageConvention,
): Promise<{ ok: true; absenceIds: string[] } | { ok: false; error: string }> {
  if (convention.stageAbsenceIds && convention.stageAbsenceIds.length > 0) {
    return { ok: true, absenceIds: convention.stageAbsenceIds };
  }

  const start = convention.schedule.periodStart?.slice(0, 10);
  const end = convention.schedule.periodEnd?.slice(0, 10);
  if (!start || !end) {
    return { ok: false, error: "Période de stage manquante." };
  }

  if (!(await isEntCoreDbEnabled())) {
    return { ok: false, error: "Base ENT indisponible." };
  }

  const etablissementId = await resolveCurrentEtablissementId();
  if (!etablissementId) {
    return { ok: false, error: "Établissement introuvable." };
  }

  const resolve = await resolveEleveIdSoft(convention, etablissementId);
  if (!resolve) {
    return {
      ok: false,
      error: "Élève introuvable pour créer l'absence stage (rattachez l'INE si besoin).",
    };
  }

  const motif = motifForConvention(convention);
  const db = getDb();
  const existing = await db
    .select({ id: vsAbsenceEleve.id, motif: vsAbsenceEleve.motif })
    .from(vsAbsenceEleve)
    .where(
      and(
        eq(vsAbsenceEleve.etablissementId, etablissementId),
        eq(vsAbsenceEleve.eleveId, resolve.eleveId),
        eq(vsAbsenceEleve.source, "accueil"),
      ),
    );
  const already = existing.find((r) => (r.motif || "").includes(convention.id.slice(-10)));
  if (already) {
    const ids = [already.id];
    await saveStageConvention({
      ...convention,
      stageAbsenceIds: ids,
      updatedAt: new Date().toISOString(),
    });
    return { ok: true, absenceIds: ids };
  }

  try {
    const created = await createAbsenceAccueilEleve(etablissementId, {
      eleveId: resolve.eleveId,
      dateDebut: start,
      dateFin: end,
      motif,
      canal: "mail",
      type: "absence",
      createdByUserId: "system:stages",
      createdByNom: "Stages & conventions",
    });
    const ids = created?.id ? [created.id] : [];
    const fresh = (await getStageConvention(convention.id)) ?? convention;
    await saveStageConvention({
      ...fresh,
      stageAbsenceIds: ids,
      updatedAt: new Date().toISOString(),
      history: [
        ...fresh.history,
        {
          at: new Date().toISOString(),
          by: "Système",
          action: "ABSENCE_STAGE_CREEE",
          note: `${start} → ${end}`,
        },
      ],
    });
    return { ok: true, absenceIds: ids };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return { ok: false, error: msg };
  }
}
