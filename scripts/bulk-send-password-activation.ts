/**
 * Envoie un e-mail d’activation / reset MDP à tous les utilisateurs sans MFA.
 *
 * Usage :
 *   npm run auth:bulk-activation -- --dry-run
 *   npm run auth:bulk-activation -- --confirm
 *   npm run auth:bulk-activation -- --confirm --etablissement=<uuid>
 *   npm run auth:bulk-activation -- --confirm --email=un@exemple.fr
 *
 * Options :
 *   --dry-run              Liste les destinataires sans envoyer (défaut si --confirm absent)
 *   --confirm              Invalide les MDP existants et envoie les e-mails
 *   --etablissement=<uuid> Limite à un établissement
 *   --email=<email>        Un seul destinataire (toujours exclu si MFA déjà activée)
 *   --delay-ms=<n>         Pause entre chaque envoi (défaut 400)
 */
import { existsSync, readFileSync } from "node:fs";
import { and, eq } from "drizzle-orm";
import { drizzle } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import * as schema from "../db/schema";
import { user } from "../db/schema";
import {
  listPasswordActivationTargets,
  sendPasswordActivationBatch,
} from "../app/lib/password-activation";

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

function argValue(name: string): string | undefined {
  const prefix = `--${name}=`;
  const hit = process.argv.find((a) => a.startsWith(prefix));
  return hit?.slice(prefix.length);
}

function hasFlag(name: string): boolean {
  return process.argv.includes(`--${name}`);
}

async function main() {
  const confirm = hasFlag("confirm");
  const dryRun = !confirm;
  const etablissementId = argValue("etablissement")?.trim();
  const singleEmail = argValue("email")?.trim().toLowerCase();
  const delayMs = Math.max(0, Number(argValue("delay-ms") ?? "400") || 400);

  const databaseUrl = process.env.DATABASE_URL?.trim();
  if (!databaseUrl) {
    console.error("DATABASE_URL manquante");
    process.exit(1);
  }
  if (!process.env.BETTER_AUTH_SECRET?.trim()) {
    console.error("BETTER_AUTH_SECRET manquant");
    process.exit(1);
  }

  const client = postgres(databaseUrl, { max: 2, prepare: false, connect_timeout: 30 });
  const db = drizzle(client, { schema });

  try {
    const rows = await listPasswordActivationTargets({
      etablissementId,
      email: singleEmail,
    });

    const skippedMfa = singleEmail
      ? []
      : await db
          .select({ email: user.email })
          .from(user)
          .where(
            and(
              eq(user.twoFactorEnabled, true),
              ...(etablissementId ? [eq(user.etablissementId, etablissementId)] : []),
            ),
          );

    console.log(
      JSON.stringify(
        {
          mode: dryRun ? "dry-run" : "confirm",
          baseUrl: process.env.BETTER_AUTH_URL ?? process.env.NEXT_PUBLIC_APP_URL,
          recipients: rows.length,
          skippedMfaAlready: skippedMfa.length,
          skippedMfaEmails: skippedMfa.map((r) => r.email),
        },
        null,
        2,
      ),
    );

    if (rows.length === 0) {
      console.log("Aucun destinataire (tous ont déjà la MFA ou filtre vide).");
      return;
    }

    console.log("\nDestinataires :");
    for (const row of rows) {
      console.log(`  - ${row.email} (${row.firstName ?? ""} ${row.lastName ?? ""})`.trim());
    }

    if (dryRun) {
      console.log(
        "\nMode simulation : relancez avec --confirm pour invalider les MDP et envoyer les e-mails.",
      );
      return;
    }

    const batch = await sendPasswordActivationBatch({
      etablissementId,
      email: singleEmail,
      delayMs,
    });

    const sent = batch.results.filter((r) => r.ok).length;
    const failed = batch.results.filter((r) => !r.ok);
    for (const result of batch.results) {
      if (result.ok) console.log(`✓ ${result.email}`);
      else console.error(`✗ ${result.email}: ${result.detail ?? result.skipped ?? "échec"}`);
    }

    console.log(
      JSON.stringify(
        {
          sent,
          failed: failed.length,
          failures: failed,
        },
        null,
        2,
      ),
    );
  } finally {
    await client.end({ timeout: 5 });
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
