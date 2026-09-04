/**
 * Fusionne les doublons élève person:… → ine:… (même nom/prénom).
 *
 * Contexte : avant le fix d’upsert du 2026-09-03, un import sans INE puis
 * un import Siècle avec INE créait deux fiches. Ce script réaffecte les
 * données liées vers la fiche INE puis supprime le stub person: (ciblé).
 *
 * Usage :
 *   NODE_TLS_REJECT_UNAUTHORIZED=0 node --import tsx scripts/merge-eleve-person-ine-duplicates.ts
 *   DRY_RUN=1 …  (affiche seulement)
 */
import fs from "node:fs";
import path from "node:path";
import postgres from "postgres";

const root = path.resolve(import.meta.dirname, "..");
const envPath = path.join(root, ".env");
const envLocalPath = path.join(root, ".env.local");
for (const p of [envLocalPath, envPath]) {
  if (!fs.existsSync(p)) continue;
  for (const line of fs.readFileSync(p, "utf8").split(/\r?\n/)) {
    const m = line.match(/^([A-Z0-9_]+)=(.*)$/);
    if (!m || process.env[m[1]]) continue;
    process.env[m[1]] = m[2].trim().replace(/^["']|["']$/g, "");
  }
}

const url = process.env.DATABASE_URL?.trim();
if (!url) {
  console.error("DATABASE_URL manquant.");
  process.exit(1);
}

const dryRun = process.env.DRY_RUN === "1" || process.env.DRY_RUN === "true";
const sql = postgres(url, {
  ssl: process.env.NODE_TLS_REJECT_UNAUTHORIZED === "0" ? { rejectUnauthorized: false } : "require",
  max: 1,
  prepare: false,
});

type Pair = {
  stub_id: string;
  keeper_id: string;
  nom: string;
  prenom: string;
  etablissement_id: string;
  keeper_ine: string;
};

const RELATED_TABLES = [
  "vs_appel_ligne",
  "vs_carnet_entree",
  "vs_sanction",
  "facture_ligne",
  "fd_fiche",
  "groupe_pedagogique_membre",
  "note_valeur",
  "note_moyenne_eleve",
  "note_competence_valeur",
] as const;

async function main() {
  const pairs = await sql<Pair[]>`
    SELECT
      p.id AS stub_id,
      i.id AS keeper_id,
      p.nom,
      p.prenom,
      p.etablissement_id,
      i.ine AS keeper_ine
    FROM eleve p
    JOIN eleve i
      ON i.etablissement_id = p.etablissement_id
     AND lower(btrim(i.nom)) = lower(btrim(p.nom))
     AND lower(btrim(i.prenom)) = lower(btrim(p.prenom))
     AND i.id <> p.id
     AND p.source_key LIKE 'person:%'
     AND (p.ine IS NULL OR btrim(p.ine) = '')
     AND i.source_key LIKE 'ine:%'
     AND i.ine IS NOT NULL
     AND btrim(i.ine) <> ''
    ORDER BY p.nom, p.prenom
  `;

  console.log(
    dryRun
      ? `[dry-run] ${pairs.length} doublon(s) person:→ine: à fusionner`
      : `Fusion de ${pairs.length} doublon(s) person:→ine:…`,
  );

  if (!pairs.length) {
    await sql.end({ timeout: 5 });
    return;
  }

  const stubCounts = new Map<string, number>();
  for (const p of pairs) {
    stubCounts.set(p.stub_id, (stubCounts.get(p.stub_id) ?? 0) + 1);
  }
  const ambiguous = [...stubCounts.entries()].filter(([, n]) => n > 1);
  if (ambiguous.length) {
    console.error("Ambiguïté (plusieurs keepers pour un stub) — abandon:", ambiguous.slice(0, 10));
    process.exit(1);
  }

  if (dryRun) {
    for (const p of pairs.slice(0, 15)) {
      console.log(
        `  - ${p.nom} ${p.prenom}  stub=${p.stub_id} → keeper=${p.keeper_id} (${p.keeper_ine})`,
      );
    }
    if (pairs.length > 15) console.log(`  … et ${pairs.length - 15} autres`);
    await sql.end({ timeout: 5 });
    return;
  }

  let merged = 0;
  const sample: string[] = [];

  await sql.begin(async (tx) => {
    for (const pair of pairs) {
      const stubId = pair.stub_id;
      const keeperId = pair.keeper_id;

      await tx`UPDATE eleve_document SET eleve_id = ${keeperId} WHERE eleve_id = ${stubId}`;
      await tx`UPDATE eleve_access_audit SET eleve_id = ${keeperId} WHERE eleve_id = ${stubId}`;
      await tx`UPDATE preinscription SET eleve_id = ${keeperId} WHERE eleve_id = ${stubId}`;

      await tx`
        UPDATE eleve_foyer_link AS stub
        SET eleve_id = ${keeperId}
        WHERE stub.eleve_id = ${stubId}
          AND NOT EXISTS (
            SELECT 1 FROM eleve_foyer_link k
            WHERE k.etablissement_id = stub.etablissement_id
              AND k.eleve_id = ${keeperId}
              AND k.foyer_id = stub.foyer_id
          )
      `;
      await tx`DELETE FROM eleve_foyer_link WHERE eleve_id = ${stubId}`;

      await tx`
        UPDATE vs_absence_eleve AS stub
        SET eleve_id = ${keeperId}
        WHERE stub.eleve_id = ${stubId}
          AND (
            stub.appel_id IS NULL
            OR NOT EXISTS (
              SELECT 1 FROM vs_absence_eleve k
              WHERE k.etablissement_id = stub.etablissement_id
                AND k.appel_id = stub.appel_id
                AND k.eleve_id = ${keeperId}
            )
          )
      `;
      await tx`DELETE FROM vs_absence_eleve WHERE eleve_id = ${stubId}`;

      for (const table of RELATED_TABLES) {
        try {
          await tx.unsafe(`UPDATE ${table} SET eleve_id = $1 WHERE eleve_id = $2`, [
            keeperId,
            stubId,
          ]);
        } catch {
          /* contrainte unique éventuelle */
        }
        try {
          await tx.unsafe(`DELETE FROM ${table} WHERE eleve_id = $1`, [stubId]);
        } catch {
          /* table absente éventuelle */
        }
      }

      await tx`DELETE FROM eleve WHERE id = ${stubId}`;
      merged += 1;
      if (sample.length < 8) {
        sample.push(`${pair.nom} ${pair.prenom} → ${pair.keeper_ine}`);
      }
    }
  });

  console.log(`OK — ${merged} stub(s) fusionné(s) vers la fiche INE.`);
  for (const s of sample) console.log(`  • ${s}`);

  const remaining = await sql`
    SELECT count(*)::int AS n FROM (
      SELECT 1
      FROM eleve
      GROUP BY etablissement_id, lower(nom), lower(prenom)
      HAVING count(*) > 1
    ) t
  `;
  console.log(`Doublons nom+prénom restants: ${remaining[0]?.n ?? "?"}`);

  await sql.end({ timeout: 5 });
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
