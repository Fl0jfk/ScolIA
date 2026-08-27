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
  etablissement,
  twoFactor,
  user,
  userMembership,
  userRole,
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

    console.log(
      JSON.stringify(
        {
          ok: true,
          email: DEV_SEED.email,
          password: DEV_SEED.password,
          totpSecret: DEV_SEED.totpSecret,
          slug: DEV_SEED.slug,
          signInUrl: "http://localhost:3000/auth/sign-in?dev_tenant=default",
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
