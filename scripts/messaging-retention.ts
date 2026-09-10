/**
 * Purge des messages messagerie plus anciens que 365 jours.
 *
 *   npx tsx --env-file=.env.local --require ./scripts/stub-server-only.cjs scripts/messaging-retention.ts
 *   npx tsx --env-file=.env.local --require ./scripts/stub-server-only.cjs scripts/messaging-retention.ts --etablissement=<uuid>
 */
import { purgeExpiredMessages } from "../app/lib/messaging/retention";

async function main() {
  const arg = process.argv.find((a) => a.startsWith("--etablissement="));
  const etabId = arg ? arg.slice("--etablissement=".length).trim() : undefined;
  const result = await purgeExpiredMessages(etabId);
  console.log(
    JSON.stringify(
      {
        ok: true,
        etablissementId: etabId ?? null,
        ...result,
      },
      null,
      2,
    ),
  );
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
