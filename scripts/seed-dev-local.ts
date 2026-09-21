/**
 * Seed idempotent pour le développement local / Cloud Agents.
 *
 * Crée :
 * - établissement slug `default`
 * - admin de test (orgAdmin) avec MDP + TOTP connus
 * - membership staff + rôle `admin`
 *
 * Usage : npm run seed:dev
 */
import { existsSync, readFileSync } from "node:fs";
import { hashPassword, symmetricEncrypt } from "better-auth/crypto";
import { and, eq } from "drizzle-orm";
import { drizzle } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import {
  account,
  anneeScolaire,
  eleve,
  etablissement,
  noteDevoir,
  noteMatiere,
  notePeriode,
  noteTypeDevoir,
  noteValeur,
  twoFactor,
  user,
  userMembership,
  userRole,
  vsAbsenceEleve,
} from "../db/schema";

function loadEnvFile(path: string) {
  if (!existsSync(path)) return;
  for (const line of readFileSync(path, "utf8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eqIdx = trimmed.indexOf("=");
    if (eqIdx <= 0) continue;
    const key = trimmed.slice(0, eqIdx);
    let value = trimmed.slice(eqIdx + 1);
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    if (!(key in process.env)) process.env[key] = value;
  }
}

loadEnvFile(".env.local");
loadEnvFile(".env");

/** Identifiants publics de démo locale — jamais en production. */
export const DEV_SEED = {
  slug: "default",
  etabName: "Instance de développement",
  dataBucket: "scola-dev",
  email: "admin@localhost.dev",
  password: "DevLocalPass1!",
  /** Secret TOTP en clair (32 car.) — chiffré avec BETTER_AUTH_SECRET avant insertion. */
  totpSecret: "DEVLOCALTOTPSECRET00000000000001",
  firstName: "Admin",
  lastName: "Local",
  userId: "dev-local-admin",
} as const;

/** Compte parent local — portail `/famille` (sans MFA pour démo justifs). */
export const DEV_PARENT_SEED = {
  email: "parent@localhost.dev",
  password: "DevParentPass1!",
  firstName: "Parent",
  lastName: "Local",
  userId: "dev-local-parent",
  childSourceKey: "brain-test:justif-famille",
  childNom: "JUSTIF",
  childPrenom: "Leo",
  childClasse: "4B",
} as const;

async function main() {
  const databaseUrl = process.env.DATABASE_URL?.trim();
  const authSecret = process.env.BETTER_AUTH_SECRET?.trim();
  if (!databaseUrl) {
    console.error("DATABASE_URL manquante");
    process.exit(1);
  }
  if (!authSecret) {
    console.error("BETTER_AUTH_SECRET manquant");
    process.exit(1);
  }

  const client = postgres(databaseUrl, { max: 2, prepare: false });
  const db = drizzle(client);

  try {
    let [etab] = await db
      .select()
      .from(etablissement)
      .where(eq(etablissement.slug, DEV_SEED.slug))
      .limit(1);

    if (!etab) {
      const [created] = await db
        .insert(etablissement)
        .values({
          slug: DEV_SEED.slug,
          name: DEV_SEED.etabName,
          dataBucket: DEV_SEED.dataBucket,
        })
        .returning();
      etab = created;
      console.log(`[seed] établissement créé: ${etab.slug} (${etab.id})`);
    } else {
      console.log(`[seed] établissement existant: ${etab.slug} (${etab.id})`);
    }

    let [u] = await db.select().from(user).where(eq(user.id, DEV_SEED.userId)).limit(1);
    if (!u) {
      [u] = await db
        .select()
        .from(user)
        .where(eq(user.email, DEV_SEED.email))
        .limit(1);
    }

    if (!u) {
      const [created] = await db
        .insert(user)
        .values({
          id: DEV_SEED.userId,
          name: `${DEV_SEED.firstName} ${DEV_SEED.lastName}`,
          email: DEV_SEED.email,
          emailVerified: true,
          etablissementId: etab.id,
          externalUserId: DEV_SEED.userId,
          firstName: DEV_SEED.firstName,
          lastName: DEV_SEED.lastName,
          platformAdmin: false,
          orgAdmin: true,
          mustChangePassword: false,
          twoFactorEnabled: true,
        })
        .returning();
      u = created;
      console.log(`[seed] utilisateur créé: ${u.email}`);
    } else {
      await db
        .update(user)
        .set({
          etablissementId: etab.id,
          emailVerified: true,
          orgAdmin: true,
          mustChangePassword: false,
          twoFactorEnabled: true,
          firstName: DEV_SEED.firstName,
          lastName: DEV_SEED.lastName,
          name: `${DEV_SEED.firstName} ${DEV_SEED.lastName}`,
          updatedAt: new Date(),
        })
        .where(eq(user.id, u.id));
      console.log(`[seed] utilisateur mis à jour: ${u.email}`);
    }

    const hashed = await hashPassword(DEV_SEED.password);
    const [existingAccount] = await db
      .select()
      .from(account)
      .where(and(eq(account.userId, u.id), eq(account.providerId, "credential")))
      .limit(1);

    if (existingAccount) {
      await db
        .update(account)
        .set({
          password: hashed,
          issuer: "local:credential",
          accountId: u.id,
          updatedAt: new Date(),
        })
        .where(eq(account.id, existingAccount.id));
    } else {
      await db.insert(account).values({
        id: crypto.randomUUID(),
        issuer: "local:credential",
        accountId: u.id,
        providerId: "credential",
        userId: u.id,
        password: hashed,
      });
    }
    console.log("[seed] mot de passe credential OK");

    const encryptedSecret = await symmetricEncrypt({
      key: authSecret,
      data: DEV_SEED.totpSecret,
    });
    const encryptedBackup = await symmetricEncrypt({
      key: authSecret,
      data: JSON.stringify(["DEV-BACKUP-CODE-01", "DEV-BACKUP-CODE-02"]),
    });

    await db.delete(twoFactor).where(eq(twoFactor.userId, u.id));
    await db.insert(twoFactor).values({
      id: crypto.randomUUID(),
      secret: encryptedSecret,
      backupCodes: encryptedBackup,
      userId: u.id,
      verified: true,
      failedVerificationCount: 0,
      lockedUntil: null,
    });
    console.log("[seed] TOTP activé (secret connu — voir AGENTS.md)");

    await db
      .insert(userMembership)
      .values({
        userId: u.id,
        etablissementId: etab.id,
        context: "staff",
        active: true,
      })
      .onConflictDoUpdate({
        target: [userMembership.userId, userMembership.etablissementId],
        set: { active: true, context: "staff", updatedAt: new Date() },
      });

    const [role] = await db
      .select()
      .from(userRole)
      .where(
        and(
          eq(userRole.userId, u.id),
          eq(userRole.etablissementId, etab.id),
          eq(userRole.role, "admin"),
        ),
      )
      .limit(1);
    if (!role) {
      await db.insert(userRole).values({
        etablissementId: etab.id,
        userId: u.id,
        role: "admin",
      });
    }

    // —— Parent démo (justifs absences /famille) ——
    let [parentUser] = await db
      .select()
      .from(user)
      .where(eq(user.id, DEV_PARENT_SEED.userId))
      .limit(1);
    if (!parentUser) {
      [parentUser] = await db
        .select()
        .from(user)
        .where(eq(user.email, DEV_PARENT_SEED.email))
        .limit(1);
    }
    if (!parentUser) {
      const [created] = await db
        .insert(user)
        .values({
          id: DEV_PARENT_SEED.userId,
          name: `${DEV_PARENT_SEED.firstName} ${DEV_PARENT_SEED.lastName}`,
          email: DEV_PARENT_SEED.email,
          emailVerified: true,
          etablissementId: etab.id,
          externalUserId: DEV_PARENT_SEED.userId,
          firstName: DEV_PARENT_SEED.firstName,
          lastName: DEV_PARENT_SEED.lastName,
          platformAdmin: false,
          orgAdmin: false,
          mustChangePassword: false,
          twoFactorEnabled: false,
        })
        .returning();
      parentUser = created;
      console.log(`[seed] parent créé: ${parentUser.email}`);
    } else {
      await db
        .update(user)
        .set({
          etablissementId: etab.id,
          emailVerified: true,
          orgAdmin: false,
          twoFactorEnabled: false,
          firstName: DEV_PARENT_SEED.firstName,
          lastName: DEV_PARENT_SEED.lastName,
          name: `${DEV_PARENT_SEED.firstName} ${DEV_PARENT_SEED.lastName}`,
          updatedAt: new Date(),
        })
        .where(eq(user.id, parentUser.id));
      console.log(`[seed] parent mis à jour: ${parentUser.email}`);
    }

    const parentHashed = await hashPassword(DEV_PARENT_SEED.password);
    const [parentAccount] = await db
      .select()
      .from(account)
      .where(and(eq(account.userId, parentUser.id), eq(account.providerId, "credential")))
      .limit(1);
    if (parentAccount) {
      await db
        .update(account)
        .set({
          password: parentHashed,
          issuer: "local:credential",
          accountId: parentUser.id,
          updatedAt: new Date(),
        })
        .where(eq(account.id, parentAccount.id));
    } else {
      await db.insert(account).values({
        id: crypto.randomUUID(),
        issuer: "local:credential",
        accountId: parentUser.id,
        providerId: "credential",
        userId: parentUser.id,
        password: parentHashed,
      });
    }
    await db.delete(twoFactor).where(eq(twoFactor.userId, parentUser.id));

    await db
      .insert(userMembership)
      .values({
        userId: parentUser.id,
        etablissementId: etab.id,
        context: "famille",
        active: true,
      })
      .onConflictDoUpdate({
        target: [userMembership.userId, userMembership.etablissementId],
        set: { active: true, context: "famille", updatedAt: new Date() },
      });

    const [parentRole] = await db
      .select()
      .from(userRole)
      .where(
        and(
          eq(userRole.userId, parentUser.id),
          eq(userRole.etablissementId, etab.id),
          eq(userRole.role, "parent"),
        ),
      )
      .limit(1);
    if (!parentRole) {
      await db.insert(userRole).values({
        etablissementId: etab.id,
        userId: parentUser.id,
        role: "parent",
      });
    }

    let [child] = await db
      .select()
      .from(eleve)
      .where(
        and(
          eq(eleve.etablissementId, etab.id),
          eq(eleve.sourceKey, DEV_PARENT_SEED.childSourceKey),
        ),
      )
      .limit(1);
    if (!child) {
      const [created] = await db
        .insert(eleve)
        .values({
          etablissementId: etab.id,
          sourceKey: DEV_PARENT_SEED.childSourceKey,
          ine: "INEJUSTIF001",
          nom: DEV_PARENT_SEED.childNom,
          prenom: DEV_PARENT_SEED.childPrenom,
          folderName: `${DEV_PARENT_SEED.childNom} ${DEV_PARENT_SEED.childPrenom}`,
          status: "inscrit",
          classe: DEV_PARENT_SEED.childClasse,
          parentEmail: DEV_PARENT_SEED.email,
        })
        .returning();
      child = created;
      console.log(`[seed] enfant justif créé: ${child.prenom} ${child.nom}`);
    } else {
      await db
        .update(eleve)
        .set({
          parentEmail: DEV_PARENT_SEED.email,
          classe: DEV_PARENT_SEED.childClasse,
          updatedAt: new Date(),
        })
        .where(eq(eleve.id, child.id));
    }

    const today = new Date().toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
    const [existingAbs] = await db
      .select()
      .from(vsAbsenceEleve)
      .where(
        and(
          eq(vsAbsenceEleve.etablissementId, etab.id),
          eq(vsAbsenceEleve.eleveId, child.id),
          eq(vsAbsenceEleve.dateDebut, today),
          eq(vsAbsenceEleve.source, "appel"),
        ),
      )
      .limit(1);
    if (!existingAbs) {
      await db.insert(vsAbsenceEleve).values({
        etablissementId: etab.id,
        eleveId: child.id,
        dateDebut: today,
        dateFin: today,
        type: "absence",
        statut: "a_traiter",
        justifie: false,
        motif: null,
        source: "appel",
        createdByNom: "seed:dev",
      });
      console.log(`[seed] absence a_traiter du jour pour ${child.prenom}`);
    }

    // —— Notes démo (saisie → parent voit sans clôture) ——
    const existingMatieres = await db
      .select()
      .from(noteMatiere)
      .where(eq(noteMatiere.etablissementId, etab.id));
    if (!existingMatieres.length) {
      for (const m of [
        { code: "MATHS", libelle: "Mathématiques" },
        { code: "FRAN", libelle: "Français" },
        { code: "HG", libelle: "Histoire-Géographie" },
        { code: "AGL1", libelle: "Anglais LV1" },
        { code: "EPS", libelle: "EPS" },
      ]) {
        await db.insert(noteMatiere).values({
          etablissementId: etab.id,
          code: m.code,
          libelle: m.libelle,
        });
      }
      console.log("[seed] matières notes créées");
    }
    const [anneeCourante] = await db
      .select()
      .from(anneeScolaire)
      .where(and(eq(anneeScolaire.etablissementId, etab.id), eq(anneeScolaire.isCurrent, true)))
      .limit(1);
    const existingPeriodes = await db
      .select()
      .from(notePeriode)
      .where(eq(notePeriode.etablissementId, etab.id));
    if (!existingPeriodes.length) {
      for (const p of [
        { code: "T1", libelle: "1er trimestre", ordre: 1 },
        { code: "T2", libelle: "2e trimestre", ordre: 2 },
        { code: "T3", libelle: "3e trimestre", ordre: 3 },
      ]) {
        await db.insert(notePeriode).values({
          etablissementId: etab.id,
          code: p.code,
          libelle: p.libelle,
          ordre: p.ordre,
          statut: "ouverte",
          anneeScolaireId: anneeCourante?.id ?? null,
        });
      }
      console.log("[seed] périodes notes créées");
    }
    for (const t of [
      { code: "DS", libelle: "Devoir surveillé" },
      { code: "DM", libelle: "Devoir maison" },
    ]) {
      await db
        .insert(noteTypeDevoir)
        .values({ etablissementId: etab.id, code: t.code, libelle: t.libelle })
        .onConflictDoNothing();
    }

    const [maths] = await db
      .select()
      .from(noteMatiere)
      .where(and(eq(noteMatiere.etablissementId, etab.id), eq(noteMatiere.code, "MATHS")))
      .limit(1);
    const [t1] = await db
      .select()
      .from(notePeriode)
      .where(and(eq(notePeriode.etablissementId, etab.id), eq(notePeriode.code, "T1")))
      .limit(1);
    if (maths && t1) {
      const [existingDevoir] = await db
        .select()
        .from(noteDevoir)
        .where(
          and(
            eq(noteDevoir.etablissementId, etab.id),
            eq(noteDevoir.libelle, "Contrôle seed JUSTIF"),
            eq(noteDevoir.classe, DEV_PARENT_SEED.childClasse),
          ),
        )
        .limit(1);
      let devoirId = existingDevoir?.id;
      if (!devoirId) {
        const [createdDevoir] = await db
          .insert(noteDevoir)
          .values({
            etablissementId: etab.id,
            matiereId: maths.id,
            periodeId: t1.id,
            classe: DEV_PARENT_SEED.childClasse,
            libelle: "Contrôle seed JUSTIF",
            dateDevoir: today,
            coefficient: "1",
            createdByUserId: DEV_SEED.userId,
          })
          .returning();
        devoirId = createdDevoir.id;
        console.log(`[seed] devoir notes créé: ${createdDevoir.libelle}`);
      }
      const [existingNote] = await db
        .select()
        .from(noteValeur)
        .where(
          and(
            eq(noteValeur.etablissementId, etab.id),
            eq(noteValeur.devoirId, devoirId),
            eq(noteValeur.eleveId, child.id),
          ),
        )
        .limit(1);
      if (!existingNote) {
        await db.insert(noteValeur).values({
          etablissementId: etab.id,
          devoirId,
          eleveId: child.id,
          valeur: "14.5",
          absent: false,
          dispense: false,
          appreciation: "Bon travail — seed démo notes famille",
        });
        console.log(`[seed] note 14.5 pour ${child.prenom} ${child.nom}`);
      }
    }

    console.log(
      JSON.stringify(
        {
          ok: true,
          email: DEV_SEED.email,
          password: DEV_SEED.password,
          totpSecret: DEV_SEED.totpSecret,
          parentEmail: DEV_PARENT_SEED.email,
          parentPassword: DEV_PARENT_SEED.password,
          parentChild: `${DEV_PARENT_SEED.childPrenom} ${DEV_PARENT_SEED.childNom}`,
          slug: DEV_SEED.slug,
          signInUrl: "http://localhost:3000/auth/sign-in?dev_tenant=default",
          familleUrl: "http://localhost:3000/famille/absences?dev_tenant=default",
          familleNotesUrl: "http://localhost:3000/famille/notes?dev_tenant=default",
          notesSaisieUrl: "http://localhost:3000/notes/saisie?dev_tenant=default",
          totpHelper: "npm run seed:dev:totp",
        },
        null,
        2,
      ),
    );
  } finally {
    await client.end({ timeout: 5 });
  }
}

const isDirectRun =
  process.argv[1]?.includes("seed-dev-local") ||
  process.argv[1]?.endsWith("seed-dev-local.ts");

if (isDirectRun) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
