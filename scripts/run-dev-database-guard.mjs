import { validateDevScriptDatabaseUrl } from "./test-database-guard.mjs";
import { loadEnvLocal } from "./load-env-local.mjs";

loadEnvLocal();
const result = validateDevScriptDatabaseUrl(process.env.DATABASE_URL);
if (!result.ok) {
  console.error(`[db-guard] ${result.reason}`);
  console.error(
    "[db-guard] seed:dev et drizzle-kit push sont limités à une base locale de dev/test.",
  );
  console.error("[db-guard] Dérogation explicite : I_KNOW_THIS_IS_NOT_PROD=1");
  process.exit(1);
}
