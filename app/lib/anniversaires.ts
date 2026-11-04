import "server-only";

import { and, asc, eq, inArray, isNotNull, notInArray, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, personnel, personnelAttr, user, userRole } from "@/db/schema";
import { drizzleEleveActifPourListes } from "@/app/lib/eleve-actif-scope";
import {
  anniversaireMonthDays,
  isAnniversaireToday,
  primaryInternalRole,
} from "@/app/lib/anniversaires-date";
import { INTRANET_ROLE_OPTIONS } from "@/app/lib/intranet-roles";
import { findFicheForAccount, isFicheDuplicateByName, personNameKey } from "@/app/lib/anniversaires-merge";

export type AnniversairesDuJour = {
  eleves: { prenom: string; nom: string; classe: string }[];
  personnels: { prenom: string; nom: string; fonction: string }[];
};

const cmp = (a: string, b: string) => a.localeCompare(b, "fr", { sensitivity: "base" });
const lc = (v: string | null | undefined) => (v ?? "").trim().toLowerCase();

function roleLabel(slug: string | null): string {
  if (!slug) return "";
  const full = INTRANET_ROLE_OPTIONS.find((o) => o.slug === slug)?.label ?? slug;
  // « Admin (gestion utilisateurs & paramètres) » -> « Admin »
  return full.replace(/\s*\(.*\)\s*$/, "").trim();
}

/**
 * Anniversaires du jour pour un établissement. Aucune date de naissance ni âge n'est renvoyé.
 * Parents exclus par construction. Personnel = comptes internes (date sur le compte, prioritaire)
 * + fiches RH actives (date `profile.birthDate`), dédoublonnés par lien de compte ou e-mail.
 * `includeEleves=false` : liste d'élèves vide (ex. maintenance seule).
 */
export async function listAnniversairesDuJour(
  etablissementId: string,
  todayKey: string,
  opts: { includeEleves?: boolean } = {},
): Promise<AnniversairesDuJour> {
  const mds = anniversaireMonthDays(todayKey);
  if (mds.length === 0) return { eleves: [], personnels: [] };
  const db = getDb();

  const eleveRows =
    opts.includeEleves === false
      ? []
      : await db
          .select({ prenom: eleve.prenom, nom: eleve.nom, classe: eleve.classe })
          .from(eleve)
          .where(
            and(
              eq(eleve.etablissementId, etablissementId),
              drizzleEleveActifPourListes(),
              inArray(sql<string>`to_char(${eleve.dateNaissance}, 'MM-DD')`, mds),
            ),
          )
          .orderBy(asc(eleve.classe), asc(eleve.nom), asc(eleve.prenom));

  // (a) Comptes internes (au moins un rôle autre que parent/eleve) dont c'est l'anniversaire.
  const internalUserIds = db
    .select({ id: userRole.userId })
    .from(userRole)
    .where(
      and(eq(userRole.etablissementId, etablissementId), notInArray(userRole.role, ["parent", "eleve"])),
    );
  const accountRows = await db
    .select({
      id: user.id,
      externalUserId: user.externalUserId,
      email: user.email,
      name: user.name,
      firstName: user.firstName,
      lastName: user.lastName,
    })
    .from(user)
    .where(
      and(
        eq(user.etablissementId, etablissementId),
        inArray(user.id, internalUserIds),
        inArray(sql<string>`to_char(${user.dateNaissance}, 'MM-DD')`, mds),
      ),
    );
  const accountIds = accountRows.map((a) => a.id);
  const rolesByUser = new Map<string, string[]>();
  if (accountIds.length > 0) {
    const roleRows = await db
      .select({ userId: userRole.userId, role: userRole.role })
      .from(userRole)
      .where(and(eq(userRole.etablissementId, etablissementId), inArray(userRole.userId, accountIds)));
    for (const r of roleRows) rolesByUser.set(r.userId, [...(rolesByUser.get(r.userId) ?? []), r.role]);
  }

  // Comptes de l'établissement ayant une date : la date du compte prime sur celle de la fiche.
  const datedUsers = await db
    .select({ id: user.id, externalUserId: user.externalUserId, email: user.email })
    .from(user)
    .where(and(eq(user.etablissementId, etablissementId), isNotNull(user.dateNaissance)));
  const datedKeys = new Set<string>();
  for (const u of datedUsers) {
    datedKeys.add(`id:${u.id}`);
    if (u.externalUserId) datedKeys.add(`id:${u.externalUserId}`);
    if (u.email) datedKeys.add(`mail:${lc(u.email)}`);
  }

  // (b) Fiches RH (actives et inactives : les inactives servent à exclure les ex-salariés).
  const persRows = await db
    .select({
      externalUserId: personnel.externalUserId,
      email: personnel.email,
      emailPro: personnel.emailPro,
      emailPerso: personnel.emailPerso,
      firstName: personnel.firstName,
      lastName: personnel.lastName,
      displayName: personnel.displayName,
      jobTitle: personnel.jobTitle,
      active: personnel.active,
      birth: personnelAttr.value,
    })
    .from(personnel)
    .leftJoin(
      personnelAttr,
      and(
        eq(personnelAttr.etablissementId, personnel.etablissementId),
        eq(personnelAttr.personnelId, personnel.id),
        eq(personnelAttr.path, "profile.birthDate"),
      ),
    )
    .where(eq(personnel.etablissementId, etablissementId));

  const ficheKeys = (f: (typeof persRows)[number]): string[] => [
    ...(f.externalUserId ? [`id:${f.externalUserId}`] : []),
    ...[f.email, f.emailPro, f.emailPerso].filter((e): e is string => !!e?.trim()).map((e) => `mail:${lc(e)}`),
  ];
  const userKeys = (u: (typeof accountRows)[number]): string[] => [
    `id:${u.id}`,
    ...(u.externalUserId ? [`id:${u.externalUserId}`] : []),
    ...(u.email ? [`mail:${lc(u.email)}`] : []),
  ];

  const ficheLites = persRows.map((f) => ({
    f,
    active: f.active !== false,
    keys: ficheKeys(f),
    nameKey: personNameKey(f.firstName, f.lastName),
    jobTitle: f.jobTitle,
  }));

  const seen = new Set<string>();
  const seenNames = new Set<string>();
  const out: { prenom: string; nom: string; fonction: string }[] = [];

  for (const a of accountRows) {
    const keys = userKeys(a);
    keys.forEach((k) => seen.add(k));
    const accNameKey = a.lastName || a.firstName ? personNameKey(a.firstName, a.lastName) : personNameKey(a.name, "");
    if (accNameKey) seenNames.add(accNameKey);
    // Ex-salarié : fiche RH liée par id/e-mail et inactive -> exclu.
    const linked = ficheLites.filter((f) => f.keys.some((k) => keys.includes(k)));
    if (linked.length > 0 && linked.every((f) => !f.active)) continue;
    const fiche = findFicheForAccount({ keys, nameKey: accNameKey }, ficheLites)?.f;
    const nom = (a.lastName ?? "").trim();
    const prenom = (a.firstName ?? "").trim() || (nom ? "" : (a.name ?? "").trim());
    const fonction = (fiche?.jobTitle ?? "").trim() || roleLabel(primaryInternalRole(rolesByUser.get(a.id) ?? []));
    if (prenom || nom) out.push({ prenom, nom, fonction });
  }

  for (const f of persRows) {
    if (f.active === false) continue;
    if (!f.birth) continue;
    // Feuille texte encodée "s:YYYY-MM-DD" (cf. ent-attr-codec).
    const raw = f.birth.startsWith("s:") ? f.birth.slice(2) : f.birth;
    if (!isAnniversaireToday(raw, todayKey)) continue;
    const keys = ficheKeys(f);
    if (keys.some((k) => seen.has(k) || datedKeys.has(k))) continue;
    if (isFicheDuplicateByName(personNameKey(f.firstName, f.lastName), seenNames)) continue;
    const nom = f.lastName.trim();
    const prenom = f.firstName.trim() || (nom ? "" : f.displayName.trim());
    if (prenom || nom) out.push({ prenom, nom, fonction: (f.jobTitle ?? "").trim() });
  }

  out.sort((a, b) => cmp(a.nom, b.nom) || cmp(a.prenom, b.prenom));

  return {
    eleves: eleveRows
      .map((r) => ({ prenom: r.prenom, nom: r.nom, classe: r.classe ?? "" }))
      .sort((a, b) => cmp(a.classe, b.classe) || cmp(a.nom, b.nom) || cmp(a.prenom, b.prenom)),
    personnels: out,
  };
}
