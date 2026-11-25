/**
 * Import des foyers — application du plan, par lots, une transaction par lot.
 *
 * Tables écrites : foyer, foyer_responsable, eleve_foyer_link (+ journal d'accès).
 * La table `eleve` n'est ni importée ni écrite ici (garanti par test).
 */

import { randomUUID } from "node:crypto";
import { and, eq, inArray } from "drizzle-orm";
import { eleveAccessAudit, eleveFoyerLink, foyer, foyerResponsable } from "@/db/schema";
import type { Plan, PlanResp } from "./plan";

export const APPLY_BATCH_SIZE = 100;

/** Sous-ensemble de l'API drizzle utilisé (permet un faux en test). */
export type FoyersWriter = {
  insert: (table: unknown) => any;
  update: (table: unknown) => any;
  delete: (table: unknown) => any;
};

export type FoyersDb = {
  transaction: <T>(fn: (tx: FoyersWriter) => Promise<T>) => Promise<T>;
};

export type ApplyContext = {
  etablissementId: string;
  actorUserId: string | null;
  importId: string;
};

export type ApplyResult = {
  /** ref du plan → identifiant réel du foyer. */
  foyerIds: Record<string, string>;
  /** Responsables créés / modifiés avec e-mail, pour le rattachement aux comptes (hors transaction). */
  respsToLink: Array<{ respId: string; email: string }>;
  batchesDone: number;
  batchesTotal: number;
  applied: {
    foyersCrees: number;
    foyersMisAJour: number;
    responsablesAjoutes: number;
    responsablesModifies: number;
    liensAjoutes: number;
    liensRetires: number;
  };
};

export class ApplyPartialError extends Error {
  constructor(
    message: string,
    readonly partial: ApplyResult,
  ) {
    super(message);
  }
}

export async function applyPlan(db: FoyersDb, plan: Plan, ctx: ApplyContext): Promise<ApplyResult> {
  const foyerByRef = new Map(plan.foyers.map((f) => [f.ref, f]));
  const addsByEleve = new Map<string, string[]>();
  for (const l of plan.linksAdd) {
    const a = addsByEleve.get(l.eleveId);
    if (a) a.push(l.foyerRef);
    else addsByEleve.set(l.eleveId, [l.foyerRef]);
  }
  const removesByEleve = new Map<string, string[]>();
  for (const l of plan.linksRemove) {
    const a = removesByEleve.get(l.eleveId);
    if (a) a.push(l.foyerId);
    else removesByEleve.set(l.eleveId, [l.foyerId]);
  }

  // Élèves à traiter : ceux dont au moins un lien ou un foyer lié change.
  const touched = plan.students.filter((s) => {
    if (addsByEleve.has(s.eleveId) || removesByEleve.has(s.eleveId)) return true;
    return s.foyerRefs.some((r) => foyerByRef.get(r)?.action !== "unchanged");
  });
  const batches: (typeof touched)[] = [];
  for (let i = 0; i < touched.length; i += APPLY_BATCH_SIZE) batches.push(touched.slice(i, i + APPLY_BATCH_SIZE));

  const result: ApplyResult = {
    foyerIds: {},
    respsToLink: [],
    batchesDone: 0,
    batchesTotal: batches.length,
    applied: {
      foyersCrees: 0,
      foyersMisAJour: 0,
      responsablesAjoutes: 0,
      responsablesModifies: 0,
      liensAjoutes: 0,
      liensRetires: 0,
    },
  };
  for (const f of plan.foyers) if (f.existingId) result.foyerIds[f.ref] = f.existingId;
  const done = new Set<string>();

  for (const batch of batches) {
    try {
      const out = await db.transaction(async (tx) => {
        const ids: Record<string, string> = {};
        const link: Array<{ respId: string; email: string }> = [];
        const doneBatch: string[] = [];
        const a = {
          foyersCrees: 0,
          foyersMisAJour: 0,
          responsablesAjoutes: 0,
          responsablesModifies: 0,
          liensAjoutes: 0,
          liensRetires: 0,
        };
        const audits: Array<typeof eleveAccessAudit.$inferInsert> = [];
        const audit = (
          resourceId: string,
          eleveId: string | null,
          action: string,
          resourceType = "fiche_foyer",
          metadata: Record<string, unknown> = {},
        ) =>
          audits.push({
            etablissementId: ctx.etablissementId,
            actorUserId: ctx.actorUserId,
            resourceType,
            resourceId,
            eleveId,
            action,
            metadata: { source: "import_excel_foyers", importId: ctx.importId, ...metadata },
          });
        const resolve = (ref: string) => result.foyerIds[ref] ?? ids[ref];

        const newFoyers: Array<typeof foyer.$inferInsert> = [];
        const newResps: Array<typeof foyerResponsable.$inferInsert> = [];
        const updatedFoyerIds: string[] = [];
        const respRow = (foyerId: string, r: PlanResp) => {
          const id = randomUUID();
          if (r.email) link.push({ respId: id, email: r.email });
          return {
            id,
            etablissementId: ctx.etablissementId,
            foyerId,
            nom: r.nom,
            prenom: r.prenom,
            email: r.email,
            telephone: r.telephone,
            autoriteParentale: r.autoriteParentale,
            contactUrgence: false,
            payeur: false,
            peutRecuperer: false,
            rang: r.rang,
          };
        };

        for (const st of batch) {
          for (const ref of st.foyerRefs) {
            if (done.has(ref) || doneBatch.includes(ref)) continue;
            const pf = foyerByRef.get(ref);
            if (!pf) continue;
            doneBatch.push(ref);
            if (pf.action === "create") {
              const id = randomUUID();
              ids[ref] = id;
              newFoyers.push({ id, etablissementId: ctx.etablissementId, label: pf.label, payeurEstFoyer: true });
              for (const r of pf.responsables) newResps.push(respRow(id, r));
              a.foyersCrees++;
              a.responsablesAjoutes += pf.responsables.length;
              audit(id, pf.eleveIds[0] ?? null, "create", "fiche_foyer", { responsables: pf.responsables.length });
            } else if (pf.action === "update" && pf.existingId) {
              for (const r of pf.responsables) {
                if (r.action === "update" && r.respId) {
                  // Jamais : payeur, userId, contactUrgence, peutRecuperer.
                  await tx
                    .update(foyerResponsable)
                    .set({
                      nom: r.nom,
                      prenom: r.prenom,
                      email: r.email,
                      telephone: r.telephone,
                      autoriteParentale: r.autoriteParentale,
                      updatedAt: new Date(),
                    })
                    .where(
                      and(
                        eq(foyerResponsable.id, r.respId),
                        eq(foyerResponsable.etablissementId, ctx.etablissementId),
                      ),
                    );
                  a.responsablesModifies++;
                  if (r.email && !r.hasUser && r.changes.includes("e-mail")) {
                    link.push({ respId: r.respId, email: r.email });
                  }
                } else if (r.action === "create") {
                  newResps.push(respRow(pf.existingId, r));
                  a.responsablesAjoutes++;
                }
              }
              updatedFoyerIds.push(pf.existingId);
              a.foyersMisAJour++;
              audit(pf.existingId, pf.eleveIds[0] ?? null, "update", "fiche_foyer", {
                modifies: pf.responsables.filter((r) => r.action === "update").length,
                ajoutes: pf.responsables.filter((r) => r.action === "create").length,
              });
            }
          }
        }
        if (newFoyers.length) await tx.insert(foyer).values(newFoyers);
        if (newResps.length) await tx.insert(foyerResponsable).values(newResps);
        if (updatedFoyerIds.length) {
          await tx
            .update(foyer)
            .set({ updatedAt: new Date() })
            .where(and(eq(foyer.etablissementId, ctx.etablissementId), inArray(foyer.id, updatedFoyerIds)));
        }

        // Liens : une insertion multi-lignes, une suppression par élève.
        const linkRows: Array<typeof eleveFoyerLink.$inferInsert> = [];
        for (const st of batch) {
          for (const ref of addsByEleve.get(st.eleveId) ?? []) {
            const foyerId = resolve(ref);
            if (!foyerId) continue;
            linkRows.push({ etablissementId: ctx.etablissementId, eleveId: st.eleveId, foyerId, relation: "principal" });
            audit(foyerId, st.eleveId, "link_add", "eleve_foyer_link");
          }
          const rem = removesByEleve.get(st.eleveId) ?? [];
          if (rem.length) {
            await tx
              .delete(eleveFoyerLink)
              .where(
                and(
                  eq(eleveFoyerLink.etablissementId, ctx.etablissementId),
                  eq(eleveFoyerLink.eleveId, st.eleveId),
                  inArray(eleveFoyerLink.foyerId, rem),
                ),
              );
            a.liensRetires += rem.length;
            for (const fid of rem) audit(fid, st.eleveId, "link_remove", "eleve_foyer_link");
          }
        }
        if (linkRows.length) {
          await tx.insert(eleveFoyerLink).values(linkRows).onConflictDoNothing();
          a.liensAjoutes += linkRows.length;
        }
        if (audits.length) await tx.insert(eleveAccessAudit).values(audits);

        return { ids, link, doneBatch, a };
      });
      // Lot validé : on publie les résultats.
      Object.assign(result.foyerIds, out.ids);
      for (const r of out.doneBatch) done.add(r);
      result.respsToLink.push(...out.link);
      for (const k of Object.keys(out.a) as Array<keyof typeof out.a>) result.applied[k] += out.a[k];
      result.batchesDone++;
    } catch (err) {
      throw new ApplyPartialError(
        `Échec au lot ${result.batchesDone + 1}/${result.batchesTotal} : ${
          err instanceof Error ? err.message : String(err)
        }. Les lots précédents sont enregistrés : refaites l'aperçu puis appliquez à nouveau (sans doublon).`,
        result,
      );
    }
  }
  return result;
}
