/**
 * Import des foyers — application du plan, par lots, une transaction par lot.
 *
 * Tables écrites : foyer, foyer_responsable, eleve_foyer_link (+ journal d'accès).
 * La table `eleve` n'est ni importée ni écrite ici (garanti par test).
 */

import { and, eq } from "drizzle-orm";
import { eleveAccessAudit, eleveFoyerLink, foyer, foyerResponsable } from "@/db/schema";
import type { Plan } from "./plan";

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
        // Résultats provisoires du lot : appliqués au résultat global seulement après commit.
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

        for (const st of batch) {
          for (const ref of st.foyerRefs) {
            if (done.has(ref) || doneBatch.includes(ref)) continue;
            const pf = foyerByRef.get(ref);
            if (!pf) continue;
            doneBatch.push(ref);
            if (pf.action === "create") {
              const [created] = await tx
                .insert(foyer)
                .values({
                  etablissementId: ctx.etablissementId,
                  label: pf.label,
                  payeurEstFoyer: true,
                })
                .returning({ id: foyer.id });
              ids[ref] = created.id;
              a.foyersCrees++;
              for (const r of pf.responsables) {
                const [row] = await tx
                  .insert(foyerResponsable)
                  .values({
                    etablissementId: ctx.etablissementId,
                    foyerId: created.id,
                    nom: r.nom,
                    prenom: r.prenom,
                    email: r.email,
                    telephone: r.telephone,
                    autoriteParentale: r.autoriteParentale,
                    contactUrgence: false,
                    payeur: false,
                    peutRecuperer: false,
                    rang: r.rang,
                  })
                  .returning({ id: foyerResponsable.id });
                a.responsablesAjoutes++;
                if (r.email) link.push({ respId: row.id, email: r.email });
              }
              audit(created.id, pf.eleveIds[0] ?? null, "create", "fiche_foyer", {
                responsables: pf.responsables.length,
              });
            } else if (pf.action === "update" && pf.existingId) {
              for (const r of pf.responsables) {
                if (r.action === "update" && r.respId) {
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
                  if (r.email && r.changes.includes("e-mail")) link.push({ respId: r.respId, email: r.email });
                } else if (r.action === "create") {
                  const [row] = await tx
                    .insert(foyerResponsable)
                    .values({
                      etablissementId: ctx.etablissementId,
                      foyerId: pf.existingId,
                      nom: r.nom,
                      prenom: r.prenom,
                      email: r.email,
                      telephone: r.telephone,
                      autoriteParentale: r.autoriteParentale,
                      contactUrgence: false,
                      payeur: false,
                      peutRecuperer: false,
                      rang: r.rang,
                    })
                    .returning({ id: foyerResponsable.id });
                  a.responsablesAjoutes++;
                  if (r.email) link.push({ respId: row.id, email: r.email });
                }
              }
              await tx
                .update(foyer)
                .set({ updatedAt: new Date() })
                .where(and(eq(foyer.id, pf.existingId), eq(foyer.etablissementId, ctx.etablissementId)));
              a.foyersMisAJour++;
              audit(pf.existingId, pf.eleveIds[0] ?? null, "update", "fiche_foyer", {
                modifies: pf.responsables.filter((r) => r.action === "update").length,
                ajoutes: pf.responsables.filter((r) => r.action === "create").length,
              });
            }
          }

          // Liens de cet élève.
          const adds = addsByEleve.get(st.eleveId) ?? [];
          const real = adds.map((ref) => resolve(ref)).filter((x): x is string => !!x);
          if (real.length) {
            await tx
              .insert(eleveFoyerLink)
              .values(
                real.map((foyerId) => ({
                  etablissementId: ctx.etablissementId,
                  eleveId: st.eleveId,
                  foyerId,
                  relation: "principal",
                })),
              )
              .onConflictDoNothing();
            a.liensAjoutes += real.length;
            for (const fid of real) audit(fid, st.eleveId, "link_add", "eleve_foyer_link");
          }
          for (const fid of removesByEleve.get(st.eleveId) ?? []) {
            await tx
              .delete(eleveFoyerLink)
              .where(
                and(
                  eq(eleveFoyerLink.etablissementId, ctx.etablissementId),
                  eq(eleveFoyerLink.eleveId, st.eleveId),
                  eq(eleveFoyerLink.foyerId, fid),
                ),
              );
            a.liensRetires++;
            audit(fid, st.eleveId, "link_remove", "eleve_foyer_link");
          }
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
        }. Les lots précédents sont enregistrés ; relancez le même fichier (idempotent).`,
        result,
      );
    }
  }
  return result;
}
