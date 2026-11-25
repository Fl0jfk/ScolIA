import "server-only";

import { randomUUID } from "node:crypto";
import { and, asc, eq, inArray, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  eleve,
  eleveFoyerLink,
  familleThread,
  foyer,
  foyerFacturation,
  foyerResponsable,
  foyerResponsableLien,
  personnel,
  user,
  userMembership,
} from "@/db/schema";
import { chunkArray } from "@/app/lib/db-in-chunks";
import { recordEleveAccessAudit } from "@/app/lib/eleve-dossier-access";
import { applyPlan, ApplyPartialError, type FoyersDb } from "./apply";
import { parseFoyersWorkbook } from "./parse";
import {
  analyzeFile,
  buildPlan,
  fileFingerprint,
  planFingerprint,
  type DbEleve,
  type DbResp,
  type ExistingData,
  type Plan,
  type PlanOptions,
} from "./plan";
import { buildReportCsv } from "./report";
import { isMissingRelationError } from "./lien-parente";
import { formatFamilleTitre, formatFoyerAdresse } from "@/app/lib/foyer-display";
import { withNamedLock } from "@/app/lib/ent-json-postgres";

export type OrphanInfo = {
  foyerId: string;
  label: string;
  facturation: boolean;
  filsMessagerie: number;
  comptes: number;
  autresEleves: number;
};

async function loadEleves(etablissementId: string): Promise<DbEleve[]> {
  const db = getDb();
  const rows = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      dateNaissance: eleve.dateNaissance,
      classe: eleve.classe,
      status: eleve.status,
      dateSortie: eleve.dateSortie,
    })
    .from(eleve)
    .where(eq(eleve.etablissementId, etablissementId));
  return rows.map((r) => ({
    ...r,
    dateNaissance: r.dateNaissance ? String(r.dateNaissance).slice(0, 10) : null,
    dateSortie: r.dateSortie ? String(r.dateSortie).slice(0, 10) : null,
  }));
}

/** E-mails du fichier : comptes du personnel et foyers de la base qui les portent déjà. */
async function loadEmailContext(
  etablissementId: string,
  fileEmails: string[],
  existingRespEmails: string[],
): Promise<Pick<ExistingData, "accountEmails" | "staffEmails" | "emailFoyers">> {
  const db = getDb();
  const emailFoyers: Record<string, string[]> = {};
  const staff = new Set<string>();
  const accounts = new Set<string>();
  const lowerIn = (col: Parameters<typeof sql>[0] | unknown, list: string[]) =>
    sql`lower(trim(${col as never})) in (${sql.join(
      list.map((e) => sql`${e}`),
      sql`, `,
    )})`;
  for (const batch of chunkArray([...new Set(fileEmails)])) {
    for (const r of await db
      .select({ email: foyerResponsable.email, foyerId: foyerResponsable.foyerId })
      .from(foyerResponsable)
      .where(and(eq(foyerResponsable.etablissementId, etablissementId), lowerIn(foyerResponsable.email, batch)))) {
      const e = (r.email ?? "").trim().toLowerCase();
      (emailFoyers[e] ??= []).push(r.foyerId);
    }
    for (const r of await db
      .select({ a: personnel.email, b: personnel.emailPro, c: personnel.emailPerso })
      .from(personnel)
      .where(
        and(
          eq(personnel.etablissementId, etablissementId),
          sql`(${lowerIn(personnel.email, batch)} or ${lowerIn(personnel.emailPro, batch)} or ${lowerIn(personnel.emailPerso, batch)})`,
        ),
      )) {
      for (const v of [r.a, r.b, r.c]) {
        const e = (v ?? "").trim().toLowerCase();
        if (e && batch.includes(e)) staff.add(e);
      }
    }
    for (const r of await db
      .select({ email: user.email })
      .from(user)
      .innerJoin(userMembership, eq(userMembership.userId, user.id))
      .where(
        and(
          eq(userMembership.etablissementId, etablissementId),
          eq(userMembership.context, "staff"),
          eq(userMembership.active, true),
          lowerIn(user.email, batch),
        ),
      )) {
      staff.add(r.email.trim().toLowerCase());
    }
  }
  for (const batch of chunkArray([...new Set(existingRespEmails)])) {
    for (const r of await db.select({ email: user.email }).from(user).where(lowerIn(user.email, batch))) {
      accounts.add(r.email.trim().toLowerCase());
    }
  }
  return { accountEmails: [...accounts], staffEmails: [...staff], emailFoyers };
}

/** Liens de parenté existants ; `null` si la table n'existe pas encore (migration non appliquée). */
async function loadLiensParente(
  etablissementId: string,
  responsableIds: string[],
): Promise<NonNullable<ExistingData["liensParente"]> | null> {
  const db = getDb();
  const out: NonNullable<ExistingData["liensParente"]> = [];
  try {
    for (const batch of chunkArray(responsableIds)) {
      out.push(
        ...(await db
          .select({
            responsableId: foyerResponsableLien.responsableId,
            eleveId: foyerResponsableLien.eleveId,
            lien: foyerResponsableLien.lien,
          })
          .from(foyerResponsableLien)
          .where(
            and(
              eq(foyerResponsableLien.etablissementId, etablissementId),
              inArray(foyerResponsableLien.responsableId, batch),
            ),
          )),
      );
    }
  } catch (err) {
    if (isMissingRelationError(err)) return null;
    throw err;
  }
  return out;
}

async function loadExisting(
  etablissementId: string,
  eleveIds: string[],
  fileEmails: string[] = [],
): Promise<ExistingData> {
  const db = getDb();
  const links: ExistingData["links"] = [];
  for (const batch of chunkArray(eleveIds)) {
    links.push(
      ...(await db
        .select({ eleveId: eleveFoyerLink.eleveId, foyerId: eleveFoyerLink.foyerId })
        .from(eleveFoyerLink)
        .where(
          and(eq(eleveFoyerLink.etablissementId, etablissementId), inArray(eleveFoyerLink.eleveId, batch)),
        )),
    );
  }
  const foyerIds = [...new Set(links.map((l) => l.foyerId))];
  const foyers: ExistingData["foyers"] = [];
  const responsables: DbResp[] = [];
  for (const batch of chunkArray(foyerIds)) {
    const [rows, fact, threads] = await Promise.all([
      db
        .select({ id: foyer.id, label: foyer.label })
        .from(foyer)
        .where(and(eq(foyer.etablissementId, etablissementId), inArray(foyer.id, batch))),
      db
        .select({ id: foyerFacturation.foyerId })
        .from(foyerFacturation)
        .where(and(eq(foyerFacturation.etablissementId, etablissementId), inArray(foyerFacturation.foyerId, batch))),
      db
        .selectDistinct({ id: familleThread.foyerId })
        .from(familleThread)
        .where(and(eq(familleThread.etablissementId, etablissementId), inArray(familleThread.foyerId, batch))),
    ]);
    const factSet = new Set(fact.map((r) => r.id));
    const threadSet = new Set(threads.map((r) => r.id));
    foyers.push(...rows.map((r) => ({ ...r, facturation: factSet.has(r.id), messagerie: threadSet.has(r.id) })));
    responsables.push(
      ...(await db
        .select({
          id: foyerResponsable.id,
          foyerId: foyerResponsable.foyerId,
          nom: foyerResponsable.nom,
          prenom: foyerResponsable.prenom,
          email: foyerResponsable.email,
          telephone: foyerResponsable.telephone,
          autoriteParentale: foyerResponsable.autoriteParentale,
          payeur: foyerResponsable.payeur,
          userId: foyerResponsable.userId,
          rang: foyerResponsable.rang,
        })
        .from(foyerResponsable)
        .where(
          and(eq(foyerResponsable.etablissementId, etablissementId), inArray(foyerResponsable.foyerId, batch)),
        )
        .orderBy(asc(foyerResponsable.rang))),
    );
  }
  const respEmails = responsables.map((r) => (r.email ?? "").trim().toLowerCase()).filter(Boolean);
  const ctx = await loadEmailContext(etablissementId, fileEmails, respEmails);
  const liensParente = await loadLiensParente(etablissementId, responsables.map((r) => r.id));
  return { foyers, responsables, links, ...ctx, liensParente };
}

/** Pour chaque foyer orphelin : facturation, fils de messagerie, comptes, autres élèves liés. */
async function describeOrphans(etablissementId: string, plan: Plan): Promise<OrphanInfo[]> {
  const ids = plan.orphelins.map((o) => o.foyerId);
  if (!ids.length) return [];
  const db = getDb();
  const fact = new Set<string>();
  const threads = new Map<string, number>();
  const comptes = new Map<string, number>();
  const liens = new Map<string, number>();
  for (const batch of chunkArray(ids)) {
    for (const r of await db
      .select({ id: foyerFacturation.foyerId })
      .from(foyerFacturation)
      .where(and(eq(foyerFacturation.etablissementId, etablissementId), inArray(foyerFacturation.foyerId, batch)))) {
      fact.add(r.id);
    }
    for (const r of await db
      .select({ id: familleThread.foyerId, n: sql<number>`count(*)::int` })
      .from(familleThread)
      .where(and(eq(familleThread.etablissementId, etablissementId), inArray(familleThread.foyerId, batch)))
      .groupBy(familleThread.foyerId)) {
      threads.set(r.id, r.n);
    }
    for (const r of await db
      .select({ id: foyerResponsable.foyerId, n: sql<number>`count(*)::int` })
      .from(foyerResponsable)
      .where(
        and(
          eq(foyerResponsable.etablissementId, etablissementId),
          inArray(foyerResponsable.foyerId, batch),
          sql`${foyerResponsable.userId} is not null`,
        ),
      )
      .groupBy(foyerResponsable.foyerId)) {
      comptes.set(r.id, r.n);
    }
    for (const r of await db
      .select({ id: eleveFoyerLink.foyerId, n: sql<number>`count(*)::int` })
      .from(eleveFoyerLink)
      .where(and(eq(eleveFoyerLink.etablissementId, etablissementId), inArray(eleveFoyerLink.foyerId, batch)))
      .groupBy(eleveFoyerLink.foyerId)) {
      liens.set(r.id, r.n);
    }
  }
  return plan.orphelins.map((o) => ({
    foyerId: o.foyerId,
    label: o.label,
    facturation: fact.has(o.foyerId),
    filsMessagerie: threads.get(o.foyerId) ?? 0,
    comptes: comptes.get(o.foyerId) ?? 0,
    // Liens restants après retrait des liens du plan.
    autresEleves: Math.max(0, (liens.get(o.foyerId) ?? 0) - o.elevesRetires.length),
  }));
}

export type PreviewResult = {
  plan: Plan;
  fileHash: string;
  fingerprint: string;
  orphans: OrphanInfo[];
};

/** Calcule le plan (lecture seule) à partir du fichier en mémoire. */
export async function computePlan(
  etablissementId: string,
  buf: Buffer,
  options: PlanOptions,
): Promise<PreviewResult> {
  const rows = parseFoyersWorkbook(buf);
  const eleves = await loadEleves(etablissementId);
  const today = new Date().toISOString().slice(0, 10);
  const analysis = analyzeFile(rows, eleves, today);
  const fileEmails = [...analysis.foyers.values()].flatMap((f) =>
    f.adults.map((a) => a.email).filter((e): e is string => !!e),
  );
  const existing = await loadExisting(etablissementId, analysis.matchedEleveIds, fileEmails);
  const plan = buildPlan(analysis, existing, options);
  const fileHash = fileFingerprint(buf);
  return {
    plan,
    fileHash,
    fingerprint: planFingerprint(plan, fileHash, options),
    orphans: await describeOrphans(etablissementId, plan),
  };
}

export class FingerprintMismatchError extends Error {}
export class ImportBusyError extends Error {}

/** Exécute `fn` sur chaque élément, `limit` à la fois. */
async function pool<T>(items: T[], limit: number, fn: (x: T) => Promise<void>): Promise<void> {
  let i = 0;
  await Promise.all(
    Array.from({ length: Math.min(limit, items.length) }, async () => {
      while (i < items.length) await fn(items[i++]!);
    }),
  );
}

const MAX_ACCOUNT_LINKS = 3000;

export async function applyImport(opts: {
  etablissementId: string;
  actorUserId: string | null;
  buf: Buffer;
  options: PlanOptions;
  expectedFingerprint: string;
  linkAccounts: boolean;
}): Promise<{
  fingerprint: string;
  importId: string;
  applied: Awaited<ReturnType<typeof applyPlan>>["applied"];
  comptesRattaches: number;
  csv: string;
  counts: Plan["counts"];
  liensParenteSautes: boolean;
}> {
  // Un seul import à la fois par établissement ; le plan est recalculé sous verrou.
  const locked = await withNamedLock("foyers-import", opts.etablissementId, 900, async () => {
    const fresh = await computePlan(opts.etablissementId, opts.buf, opts.options);
    if (fresh.fingerprint !== opts.expectedFingerprint) {
      throw new FingerprintMismatchError(
        "Le fichier ou les données ont changé depuis l'aperçu. Relancez l'aperçu avant d'appliquer.",
      );
    }
    const importId = randomUUID();
    const db = getDb();
    const ctx = { etablissementId: opts.etablissementId, actorUserId: opts.actorUserId, importId };
    const audit = (action: string, metadata: Record<string, unknown>) =>
      recordEleveAccessAudit({
        etablissementId: opts.etablissementId,
        actorUserId: opts.actorUserId,
        resourceType: "foyers_import",
        resourceId: importId,
        action,
        metadata: { fingerprint: fresh.fingerprint, ...metadata },
      });
    await audit("import_start", {
      keepOldLinks: opts.options.keepOldLinks,
      removeProtectedLinks: !!opts.options.removeProtectedLinks,
      writeSuspectEmails: !!opts.options.writeSuspectEmails,
      counts: fresh.plan.counts,
    });

    let result: Awaited<ReturnType<typeof applyPlan>>;
    try {
      result = await applyPlan(db as unknown as FoyersDb, fresh.plan, ctx);
    } catch (err) {
      await audit("import_failed", {
        applied: err instanceof ApplyPartialError ? err.partial.applied : null,
        error: err instanceof Error ? err.message : String(err),
      }).catch(() => undefined);
      throw err;
    }

    // Rattachement aux comptes existants (option), après les lots, borné et jamais sur un compte déjà lié.
    let comptesRattaches = 0;
    if (opts.linkAccounts && result.respsToLink.length) {
      const { linkFoyerResponsableToUserAccountIfUnlinked } = await import(
        "@/app/lib/nomenclature-import/link-responsable-user"
      );
      await pool(result.respsToLink.slice(0, MAX_ACCOUNT_LINKS), 6, async (r) => {
        try {
          if (await linkFoyerResponsableToUserAccountIfUnlinked(opts.etablissementId, r.respId, r.email)) {
            comptesRattaches++;
          }
        } catch (err) {
          console.warn("[foyers-import] rattachement compte", err);
        }
      });
    }

    await audit("import", {
      linkAccounts: opts.linkAccounts,
      counts: fresh.plan.counts,
      applied: result.applied,
      comptesRattaches,
      orphelins: fresh.orphans.map((o) => o.foyerId),
      liensAVerifier: fresh.plan.liensAVerifier.length,
    });

    return {
      fingerprint: fresh.fingerprint,
      importId,
      applied: result.applied,
      comptesRattaches,
      csv: buildReportCsv(fresh.plan, result.foyerIds),
      counts: fresh.plan.counts,
      liensParenteSautes: result.liensParenteSautes || fresh.plan.liensParenteIndisponible,
    };
  });
  if (locked.busy) throw new ImportBusyError("Import déjà en cours pour cet établissement.");
  return locked.value;
}

/* ───────── Vue « Foyers » (lecture seule) ───────── */

export type FoyerListItem = {
  id: string;
  label: string;
  /** « Famille de Prénom NOM » (responsable de rang 1). */
  titre: string;
  adresse: string | null;
  responsables: Array<{
    id: string;
    nom: string;
    prenom: string;
    email: string | null;
    telephone: string | null;
    autoriteParentale: boolean;
    compteFamille: boolean;
  }>;
  enfants: Array<{ id: string; nom: string; prenom: string; classe: string | null }>;
};

export async function listFoyersWithChildren(
  etablissementId: string,
  opts: { q?: string; limit?: number; offset?: number; foyerIds?: string[] },
): Promise<{ items: FoyerListItem[]; total: number }> {
  const db = getDb();
  const foyers = await db
    .select({
      id: foyer.id,
      label: foyer.label,
      adresse: foyer.adresse,
      codePostal: foyer.codePostal,
      ville: foyer.ville,
    })
    .from(foyer)
    .where(
      opts.foyerIds
        ? and(eq(foyer.etablissementId, etablissementId), inArray(foyer.id, opts.foyerIds))
        : eq(foyer.etablissementId, etablissementId),
    );
  const ids = foyers.map((f) => f.id);
  const resps: Array<{
    id: string;
    foyerId: string;
    nom: string;
    prenom: string;
    email: string | null;
    telephone: string | null;
    autoriteParentale: boolean;
    userId: string | null;
    rang: number;
  }> = [];
  const kids: Array<{ foyerId: string; id: string; nom: string; prenom: string; classe: string | null }> = [];
  for (const batch of chunkArray(ids)) {
    resps.push(
      ...(await db
        .select({
          id: foyerResponsable.id,
          foyerId: foyerResponsable.foyerId,
          nom: foyerResponsable.nom,
          prenom: foyerResponsable.prenom,
          email: foyerResponsable.email,
          telephone: foyerResponsable.telephone,
          autoriteParentale: foyerResponsable.autoriteParentale,
          userId: foyerResponsable.userId,
          rang: foyerResponsable.rang,
        })
        .from(foyerResponsable)
        .where(
          and(eq(foyerResponsable.etablissementId, etablissementId), inArray(foyerResponsable.foyerId, batch)),
        )
        .orderBy(asc(foyerResponsable.rang))),
    );
    kids.push(
      ...(await db
        .select({
          foyerId: eleveFoyerLink.foyerId,
          id: eleve.id,
          nom: eleve.nom,
          prenom: eleve.prenom,
          classe: eleve.classe,
        })
        .from(eleveFoyerLink)
        .innerJoin(eleve, eq(eleve.id, eleveFoyerLink.eleveId))
        .where(and(eq(eleveFoyerLink.etablissementId, etablissementId), inArray(eleveFoyerLink.foyerId, batch)))
        .orderBy(asc(eleve.nom), asc(eleve.prenom))),
    );
  }
  const rangOf = new Map(resps.map((r) => [r.id, r.rang]));
  const respBy = new Map<string, FoyerListItem["responsables"]>();
  for (const r of resps) {
    const l = respBy.get(r.foyerId) ?? [];
    l.push({
      id: r.id,
      nom: r.nom,
      prenom: r.prenom,
      email: r.email,
      telephone: r.telephone,
      autoriteParentale: r.autoriteParentale,
      compteFamille: !!r.userId,
    });
    respBy.set(r.foyerId, l);
  }
  const kidBy = new Map<string, FoyerListItem["enfants"]>();
  for (const k of kids) {
    const l = kidBy.get(k.foyerId) ?? [];
    l.push({ id: k.id, nom: k.nom, prenom: k.prenom, classe: k.classe });
    kidBy.set(k.foyerId, l);
  }
  let items: FoyerListItem[] = foyers.map((f) => {
    const responsables = respBy.get(f.id) ?? [];
    return {
      id: f.id,
      label: f.label,
      titre: formatFamilleTitre({
        label: f.label,
        responsables: responsables.map((r) => ({ prenom: r.prenom, nom: r.nom, rang: rangOf.get(r.id) })),
      }),
      adresse: formatFoyerAdresse(f) || null,
      responsables,
      enfants: kidBy.get(f.id) ?? [],
    };
  });
  const q = (opts.q ?? "").trim();
  if (q) {
    const { normName } = await import("./normalize");
    const nq = normName(q);
    items = items.filter((it) => {
      const hay = normName(
        [
          it.label,
          it.titre,
          ...it.responsables.flatMap((r) => [r.nom, r.prenom, r.email ?? "", r.telephone ?? ""]),
          ...it.enfants.flatMap((e) => [e.nom, e.prenom]),
        ].join(" "),
      );
      return hay.includes(nq);
    });
  }
  items.sort((a, b) => a.titre.localeCompare(b.titre, "fr"));
  const total = items.length;
  const offset = Math.max(0, opts.offset ?? 0);
  const limit = Math.min(200, Math.max(1, opts.limit ?? 50));
  return { items: items.slice(offset, offset + limit), total };
}
