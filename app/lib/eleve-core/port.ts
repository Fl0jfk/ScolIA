import "server-only";

import { and, desc, eq, inArray } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  eleve,
  eleveFoyerLink,
  eleveRegimePeriode,
  eleveScolarite,
  foyer,
} from "@/db/schema";
import { METIER_EVENT_TYPES } from "@/app/lib/eleve-core/events";
import { runEleveCoreHooks } from "@/app/lib/eleve-core/hooks";
import {
  assertDifferentYears,
  assertIsoDate,
  EleveCoreError,
  parseScolariteStatut,
  planRegimeCutover,
  regimeAtDate,
  regimeImpliesDemiPension,
  storedRegimeLabel,
  type RegimePeriodInput,
} from "@/app/lib/eleve-core/invariants";
import { recordMetierEvent } from "@/app/lib/eleve-core/journal";
import {
  ensureAnneeScolaireByLabel,
  ensureCurrentAnneeId,
  ensureNextAnneeId,
} from "@/app/lib/eleve-core/years";

export type EleveCoreWriteOpts = {
  actorUserId?: string | null;
  skipHooks?: boolean;
};

export type EleveCoreSnapshot = {
  id: string;
  etablissementId: string;
  ine: string | null;
  nom: string;
  prenom: string;
  status: string;
  classe: string | null;
  siteId: string | null;
  anneeScolaireId: string | null;
  regime: string | null;
  scolariteId: string | null;
  prevue: {
    scolariteId: string;
    classe: string | null;
    siteId: string | null;
    anneeScolaireId: string | null;
  } | null;
};

async function emit(
  event: Parameters<typeof recordMetierEvent>[0],
  opts?: EleveCoreWriteOpts,
): Promise<void> {
  await recordMetierEvent(event);
  await runEleveCoreHooks(event, { skipHooks: opts?.skipHooks });
}

export async function getEleve(opts: {
  etablissementId: string;
  eleveId: string;
}): Promise<EleveCoreSnapshot | null> {
  const db = getDb();
  const [row] = await db
    .select({
      id: eleve.id,
      etablissementId: eleve.etablissementId,
      ine: eleve.ine,
      nom: eleve.nom,
      prenom: eleve.prenom,
      status: eleve.status,
      classePlat: eleve.classe,
      regimePlat: eleve.regime,
    })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, opts.etablissementId), eq(eleve.id, opts.eleveId)))
    .limit(1);
  if (!row) return null;

  const scolarites = await db
    .select()
    .from(eleveScolarite)
    .where(
      and(
        eq(eleveScolarite.etablissementId, opts.etablissementId),
        eq(eleveScolarite.eleveId, opts.eleveId),
        inArray(eleveScolarite.statut, ["en_cours", "prevue"]),
      ),
    )
    .orderBy(desc(eleveScolarite.updatedAt));

  const enCours = scolarites.find((s) => s.statut === "en_cours") ?? null;
  const prevue = scolarites.find((s) => s.statut === "prevue") ?? null;

  let regime = storedRegimeLabel(row.regimePlat);
  if (enCours) {
    const periods = await listRegimePeriodes({
      etablissementId: opts.etablissementId,
      scolariteId: enCours.id,
    });
    const today = new Date().toISOString().slice(0, 10);
    regime = regimeAtDate(periods, today) ?? regime;
  }

  return {
    id: row.id,
    etablissementId: row.etablissementId,
    ine: row.ine,
    nom: row.nom,
    prenom: row.prenom,
    status: row.status,
    classe: enCours?.classe ?? row.classePlat,
    siteId: enCours?.siteId ?? null,
    anneeScolaireId: enCours?.anneeScolaireId ?? null,
    regime,
    scolariteId: enCours?.id ?? null,
    prevue: prevue
      ? {
          scolariteId: prevue.id,
          classe: prevue.classe,
          siteId: prevue.siteId,
          anneeScolaireId: prevue.anneeScolaireId,
        }
      : null,
  };
}

export async function listRegimePeriodes(opts: {
  etablissementId: string;
  scolariteId: string;
}): Promise<RegimePeriodInput[]> {
  const db = getDb();
  const rows = await db
    .select({
      regime: eleveRegimePeriode.regime,
      dateDebut: eleveRegimePeriode.dateDebut,
      dateFin: eleveRegimePeriode.dateFin,
    })
    .from(eleveRegimePeriode)
    .where(
      and(
        eq(eleveRegimePeriode.etablissementId, opts.etablissementId),
        eq(eleveRegimePeriode.scolariteId, opts.scolariteId),
      ),
    );
  return rows.map((r) => ({
    regime: r.regime,
    dateDebut: String(r.dateDebut),
    dateFin: r.dateFin ? String(r.dateFin) : null,
  }));
}

export async function getRegimeAtDate(opts: {
  etablissementId: string;
  eleveId: string;
  on: string;
}): Promise<string | null> {
  assertIsoDate(opts.on, "date");
  const db = getDb();
  const rows = await db
    .select({
      regime: eleveRegimePeriode.regime,
      dateDebut: eleveRegimePeriode.dateDebut,
      dateFin: eleveRegimePeriode.dateFin,
    })
    .from(eleveRegimePeriode)
    .where(
      and(
        eq(eleveRegimePeriode.etablissementId, opts.etablissementId),
        eq(eleveRegimePeriode.eleveId, opts.eleveId),
      ),
    );
  return regimeAtDate(
    rows.map((r) => ({
      regime: r.regime,
      dateDebut: String(r.dateDebut),
      dateFin: r.dateFin ? String(r.dateFin) : null,
    })),
    opts.on,
  );
}

async function findActiveSameYear(opts: {
  etablissementId: string;
  eleveId: string;
  anneeScolaireId: string;
}): Promise<{ id: string; statut: string } | null> {
  const db = getDb();
  const [row] = await db
    .select({ id: eleveScolarite.id, statut: eleveScolarite.statut })
    .from(eleveScolarite)
    .where(
      and(
        eq(eleveScolarite.etablissementId, opts.etablissementId),
        eq(eleveScolarite.eleveId, opts.eleveId),
        eq(eleveScolarite.anneeScolaireId, opts.anneeScolaireId),
        inArray(eleveScolarite.statut, ["en_cours", "prevue"]),
      ),
    )
    .limit(1);
  return row ?? null;
}

async function findEnCours(opts: {
  etablissementId: string;
  eleveId: string;
}): Promise<{
  id: string;
  classe: string | null;
  siteId: string | null;
  anneeScolaireId: string | null;
  statut: string;
  demiPension: boolean;
} | null> {
  const db = getDb();
  const [row] = await db
    .select({
      id: eleveScolarite.id,
      classe: eleveScolarite.classe,
      siteId: eleveScolarite.siteId,
      anneeScolaireId: eleveScolarite.anneeScolaireId,
      statut: eleveScolarite.statut,
      demiPension: eleveScolarite.demiPension,
    })
    .from(eleveScolarite)
    .where(
      and(
        eq(eleveScolarite.etablissementId, opts.etablissementId),
        eq(eleveScolarite.eleveId, opts.eleveId),
        eq(eleveScolarite.statut, "en_cours"),
      ),
    )
    .limit(1);
  return row ?? null;
}

async function closeEnCours(
  row: NonNullable<Awaited<ReturnType<typeof findEnCours>>>,
  opts: { etablissementId: string; eleveId: string; reason: string },
  write?: EleveCoreWriteOpts,
): Promise<void> {
  const db = getDb();
  await db
    .update(eleveScolarite)
    .set({ statut: "terminee", updatedAt: new Date() })
    .where(eq(eleveScolarite.id, row.id));
  await emit(
    {
      etablissementId: opts.etablissementId,
      type: METIER_EVENT_TYPES.SCOLARITE_CLOSED,
      aggregate: "scolarite",
      aggregateId: row.id,
      eleveId: opts.eleveId,
      payload: { reason: opts.reason, classeAvant: row.classe },
      actorUserId: write?.actorUserId,
    },
    write,
  );
}

/**
 * Pose / met à jour la classe de l’année (une seule `en_cours`).
 * Changement 4e B → 4e C : update de la même ligne.
 */
export async function applyClasseCourante(
  opts: {
    etablissementId: string;
    eleveId: string;
    classe: string;
    siteId?: string | null;
    anneeScolaireId?: string | null;
    etablissementPrecedent?: string | null;
    eleveStatus?: string | null;
  },
  write?: EleveCoreWriteOpts,
): Promise<{ scolariteId: string }> {
  const classe = opts.classe.trim();
  if (!classe) throw new EleveCoreError("INVALID_CLASSE", "Classe requise.");

  const db = getDb();
  const anneeId = opts.anneeScolaireId?.trim() || (await ensureCurrentAnneeId(opts.etablissementId));
  const asTerminee = opts.eleveStatus === "ancien" || opts.eleveStatus === "archive";
  const enCours = await findEnCours({
    etablissementId: opts.etablissementId,
    eleveId: opts.eleveId,
  });
  const sameYear = await findActiveSameYear({
    etablissementId: opts.etablissementId,
    eleveId: opts.eleveId,
    anneeScolaireId: anneeId,
  });

  if (asTerminee) {
    if (enCours) {
      await closeEnCours(
        enCours,
        { etablissementId: opts.etablissementId, eleveId: opts.eleveId, reason: "status_ancien" },
        write,
      );
    }
    return { scolariteId: enCours?.id ?? sameYear?.id ?? "" };
  }

  if (enCours && enCours.anneeScolaireId && enCours.anneeScolaireId !== anneeId) {
    await closeEnCours(
      enCours,
      {
        etablissementId: opts.etablissementId,
        eleveId: opts.eleveId,
        reason: "nouvelle_annee_en_cours",
      },
      write,
    );
  }

  const targetId =
    sameYear?.id ?? (enCours && enCours.anneeScolaireId === anneeId ? enCours.id : null);

  if (targetId) {
    const beforeClasse = enCours && enCours.id === targetId ? enCours.classe : null;
    const fromPrevue = sameYear?.statut === "prevue";
    await db
      .update(eleveScolarite)
      .set({
        statut: "en_cours",
        classe,
        ...(opts.siteId !== undefined ? { siteId: opts.siteId } : {}),
        updatedAt: new Date(),
      })
      .where(eq(eleveScolarite.id, targetId));
    await db
      .update(eleve)
      .set({ classe, updatedAt: new Date(), status: "inscrit" })
      .where(and(eq(eleve.etablissementId, opts.etablissementId), eq(eleve.id, opts.eleveId)));
    if (fromPrevue) {
      await emit(
        {
          etablissementId: opts.etablissementId,
          type: METIER_EVENT_TYPES.SCOLARITE_OPENED,
          aggregate: "scolarite",
          aggregateId: targetId,
          eleveId: opts.eleveId,
          payload: { classe, from: "prevue", anneeScolaireId: anneeId },
          actorUserId: write?.actorUserId,
        },
        write,
      );
    } else if (beforeClasse !== classe) {
      await emit(
        {
          etablissementId: opts.etablissementId,
          type: METIER_EVENT_TYPES.SCOLARITE_CLASSE_CHANGED,
          aggregate: "scolarite",
          aggregateId: targetId,
          eleveId: opts.eleveId,
          payload: { before: beforeClasse, after: classe, siteId: opts.siteId ?? null },
          actorUserId: write?.actorUserId,
        },
        write,
      );
    }
    return { scolariteId: targetId };
  }

  const [created] = await db
    .insert(eleveScolarite)
    .values({
      etablissementId: opts.etablissementId,
      eleveId: opts.eleveId,
      anneeScolaireId: anneeId,
      classe,
      siteId: opts.siteId ?? null,
      statut: "en_cours",
      etablissementPrecedent: opts.etablissementPrecedent ?? null,
    })
    .returning({ id: eleveScolarite.id });

  await db
    .update(eleve)
    .set({ classe, updatedAt: new Date(), status: "inscrit" })
    .where(and(eq(eleve.etablissementId, opts.etablissementId), eq(eleve.id, opts.eleveId)));

  await emit(
    {
      etablissementId: opts.etablissementId,
      type: METIER_EVENT_TYPES.SCOLARITE_OPENED,
      aggregate: "scolarite",
      aggregateId: created.id,
      eleveId: opts.eleveId,
      payload: { classe, statut: "en_cours", anneeScolaireId: anneeId, siteId: opts.siteId ?? null },
      actorUserId: write?.actorUserId,
    },
    write,
  );
  return { scolariteId: created.id };
}

/** Prépare l’année suivante (autre classe / autre site du groupe). Ne change pas la classe actuelle. */
export async function openPrevueScolarite(
  opts: {
    etablissementId: string;
    eleveId: string;
    classe?: string | null;
    siteId?: string | null;
    anneeScolaireId?: string | null;
    etablissementPrecedent?: string | null;
    demiPension?: boolean;
  },
  write?: EleveCoreWriteOpts,
): Promise<{ scolariteId: string }> {
  const db = getDb();
  const anneeId = opts.anneeScolaireId?.trim() || (await ensureNextAnneeId(opts.etablissementId));
  const enCours = await findEnCours({
    etablissementId: opts.etablissementId,
    eleveId: opts.eleveId,
  });
  assertDifferentYears(enCours?.anneeScolaireId ?? null, anneeId);

  const sameYear = await findActiveSameYear({
    etablissementId: opts.etablissementId,
    eleveId: opts.eleveId,
    anneeScolaireId: anneeId,
  });
  if (sameYear) {
    await db
      .update(eleveScolarite)
      .set({
        ...(opts.classe !== undefined ? { classe: opts.classe.trim() || null } : {}),
        ...(opts.siteId !== undefined ? { siteId: opts.siteId } : {}),
        statut: "prevue",
        ...(opts.etablissementPrecedent !== undefined
          ? { etablissementPrecedent: opts.etablissementPrecedent }
          : {}),
        ...(opts.demiPension !== undefined ? { demiPension: opts.demiPension } : {}),
        updatedAt: new Date(),
      })
      .where(eq(eleveScolarite.id, sameYear.id));
    await emit(
      {
        etablissementId: opts.etablissementId,
        type: METIER_EVENT_TYPES.SCOLARITE_CLASSE_CHANGED,
        aggregate: "scolarite",
        aggregateId: sameYear.id,
        eleveId: opts.eleveId,
        payload: { statut: "prevue", classe: opts.classe ?? null, anneeScolaireId: anneeId },
        actorUserId: write?.actorUserId,
      },
      write,
    );
    return { scolariteId: sameYear.id };
  }

  const [created] = await db
    .insert(eleveScolarite)
    .values({
      etablissementId: opts.etablissementId,
      eleveId: opts.eleveId,
      anneeScolaireId: anneeId,
      classe: opts.classe?.trim() || null,
      siteId: opts.siteId ?? null,
      statut: "prevue",
      demiPension: Boolean(opts.demiPension),
      etablissementPrecedent: opts.etablissementPrecedent ?? null,
    })
    .returning({ id: eleveScolarite.id });

  await emit(
    {
      etablissementId: opts.etablissementId,
      type: METIER_EVENT_TYPES.SCOLARITE_OPENED,
      aggregate: "scolarite",
      aggregateId: created.id,
      eleveId: opts.eleveId,
      payload: {
        statut: "prevue",
        classe: opts.classe ?? null,
        anneeScolaireId: anneeId,
        siteId: opts.siteId ?? null,
      },
      actorUserId: write?.actorUserId,
    },
    write,
  );
  return { scolariteId: created.id };
}

export async function applyRegimeChange(
  opts: {
    etablissementId: string;
    eleveId: string;
    regime: string;
    effectiveOn: string;
    scolariteId?: string;
  },
  write?: EleveCoreWriteOpts,
): Promise<{ noop: boolean; scolariteId: string }> {
  const label = storedRegimeLabel(opts.regime);
  if (!label) throw new EleveCoreError("INVALID_REGIME", "Régime requis.");
  const effectiveOn = assertIsoDate(opts.effectiveOn, "date d’effet");

  const db = getDb();
  let scolariteId = opts.scolariteId;
  if (!scolariteId) {
    const enCours = await findEnCours({
      etablissementId: opts.etablissementId,
      eleveId: opts.eleveId,
    });
    if (!enCours) {
      throw new EleveCoreError(
        "NO_SCOLARITE",
        "Aucune scolarité en cours — posez la classe avant le régime.",
      );
    }
    scolariteId = enCours.id;
  }

  const [open] = await db
    .select({
      id: eleveRegimePeriode.id,
      regime: eleveRegimePeriode.regime,
      dateDebut: eleveRegimePeriode.dateDebut,
      dateFin: eleveRegimePeriode.dateFin,
    })
    .from(eleveRegimePeriode)
    .where(
      and(
        eq(eleveRegimePeriode.etablissementId, opts.etablissementId),
        eq(eleveRegimePeriode.scolariteId, scolariteId),
        eq(eleveRegimePeriode.eleveId, opts.eleveId),
      ),
    )
    .orderBy(desc(eleveRegimePeriode.dateDebut));

  const openPeriod =
    open && !open.dateFin
      ? {
          regime: open.regime,
          dateDebut: String(open.dateDebut),
          dateFin: null as string | null,
        }
      : null;

  const plan = planRegimeCutover({
    open: openPeriod,
    nextRegime: label,
    effectiveOn,
  });
  if (plan.noop) return { noop: true, scolariteId };

  if (open && !open.dateFin && plan.closeDateFin) {
    await db
      .update(eleveRegimePeriode)
      .set({ dateFin: plan.closeDateFin, updatedAt: new Date() })
      .where(eq(eleveRegimePeriode.id, open.id));
  }

  await db.insert(eleveRegimePeriode).values({
    etablissementId: opts.etablissementId,
    eleveId: opts.eleveId,
    scolariteId,
    regime: plan.next.regime,
    dateDebut: plan.next.dateDebut,
    dateFin: null,
  });

  await db
    .update(eleve)
    .set({ regime: plan.next.regime, updatedAt: new Date() })
    .where(and(eq(eleve.etablissementId, opts.etablissementId), eq(eleve.id, opts.eleveId)));
  await db
    .update(eleveScolarite)
    .set({
      demiPension: regimeImpliesDemiPension(plan.next.regime),
      updatedAt: new Date(),
    })
    .where(eq(eleveScolarite.id, scolariteId));

  await emit(
    {
      etablissementId: opts.etablissementId,
      type: METIER_EVENT_TYPES.ELEVE_REGIME_CHANGED,
      aggregate: "regime",
      aggregateId: scolariteId,
      eleveId: opts.eleveId,
      payload: {
        before: openPeriod?.regime ?? null,
        after: plan.next.regime,
        effectiveOn,
        closedOn: plan.closeDateFin,
      },
      actorUserId: write?.actorUserId,
    },
    write,
  );
  return { noop: false, scolariteId };
}

/** Import / rattrapage : aligne scolarité courante + période ouverte sans inventer de dates de coupure. */
export async function syncScolariteCouranteFromPlat(
  opts: {
    etablissementId: string;
    eleveId: string;
    classe: string | null;
    regime?: string | null;
    status?: string | null;
    siteId?: string | null;
  },
  write?: EleveCoreWriteOpts,
): Promise<void> {
  const classe = opts.classe?.trim() || null;
  if (!classe) return;
  const { scolariteId } = await applyClasseCourante(
    {
      etablissementId: opts.etablissementId,
      eleveId: opts.eleveId,
      classe,
      siteId: opts.siteId,
      eleveStatus: opts.status,
    },
    { ...write, skipHooks: true },
  );
  const regime = storedRegimeLabel(opts.regime ?? null);
  if (!regime) return;
  const db = getDb();
  const [open] = await db
    .select({ id: eleveRegimePeriode.id })
    .from(eleveRegimePeriode)
    .where(
      and(
        eq(eleveRegimePeriode.scolariteId, scolariteId),
        eq(eleveRegimePeriode.etablissementId, opts.etablissementId),
      ),
    )
    .limit(1);
  const today = new Date().toISOString().slice(0, 10);
  if (!open) {
    await applyRegimeChange(
      {
        etablissementId: opts.etablissementId,
        eleveId: opts.eleveId,
        regime,
        effectiveOn: today,
        scolariteId,
      },
      { ...write, skipHooks: true },
    );
    return;
  }
  await applyRegimeChange(
    {
      etablissementId: opts.etablissementId,
      eleveId: opts.eleveId,
      regime,
      effectiveOn: today,
      scolariteId,
    },
    { ...write, skipHooks: write?.skipHooks ?? true },
  );
}

export async function recordFoyerChanged(
  opts: {
    etablissementId: string;
    eleveId: string;
    foyerId: string;
    action: string;
  },
  write?: EleveCoreWriteOpts,
): Promise<void> {
  await emit(
    {
      etablissementId: opts.etablissementId,
      type: METIER_EVENT_TYPES.FOYER_CHANGED,
      aggregate: "foyer",
      aggregateId: opts.foyerId,
      eleveId: opts.eleveId,
      payload: { action: opts.action },
      actorUserId: write?.actorUserId,
    },
    write,
  );
}

export async function listFoyerLinks(opts: {
  etablissementId: string;
  eleveId: string;
}): Promise<{ foyerId: string; label: string; relation: string }[]> {
  const db = getDb();
  const rows = await db
    .select({
      foyerId: eleveFoyerLink.foyerId,
      label: foyer.label,
      relation: eleveFoyerLink.relation,
    })
    .from(eleveFoyerLink)
    .innerJoin(foyer, eq(foyer.id, eleveFoyerLink.foyerId))
    .where(
      and(
        eq(eleveFoyerLink.etablissementId, opts.etablissementId),
        eq(eleveFoyerLink.eleveId, opts.eleveId),
      ),
    );
  return rows;
}

export { ensureAnneeScolaireByLabel, ensureCurrentAnneeId, ensureNextAnneeId, parseScolariteStatut };
