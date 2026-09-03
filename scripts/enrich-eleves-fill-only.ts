/**
 * Enrichit le référentiel élèves (Excel ou payload JSON)
 * — Match INE puis nom/prénom
 * — Remplit UNIQUEMENT les champs vides (jamais d'écrasement)
 * — N'ajoute aucun nouvel élève
 * — Ne supprime personne
 *
 * Usage :
 *   npx tsx --require ./scripts/stub-server-only.cjs scripts/enrich-eleves-fill-only.ts \
 *     --file="C:/Users/.../Classeur.xlsx" \
 *     --tenant=la-providence-nicolas-barre \
 *     [--dry-run]
 *
 *   npx tsx --require ./scripts/stub-server-only.cjs scripts/enrich-eleves-fill-only.ts \
 *     --payload=scripts/_enrich-payload.json
 */
import { existsSync, readFileSync } from "node:fs";
import { and, eq } from "drizzle-orm";
import type { EleveConfig } from "../app/lib/eleves-config";
import { normalizeEleveDateNaissance } from "../app/lib/eleves-config";
import { parseElevesExcelBuffer } from "../app/lib/eleves-import";
import { listElevesFromDb } from "../app/lib/ent-core-db";
import { closeDb, getDb, isDatabaseConfigured } from "../db/index";
import { eleve, etablissement } from "../db/schema";

function loadEnvFile(path: string) {
  if (!existsSync(path)) return;
  for (const line of readFileSync(path, "utf8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const i = trimmed.indexOf("=");
    if (i < 0) continue;
    const key = trimmed.slice(0, i).trim();
    let val = trimmed.slice(i + 1).trim();
    if (
      (val.startsWith('"') && val.endsWith('"')) ||
      (val.startsWith("'") && val.endsWith("'"))
    ) {
      val = val.slice(1, -1);
    }
    if (!(key in process.env)) process.env[key] = val;
  }
}

loadEnvFile(".env.local");
loadEnvFile(".env");

function argValue(name: string): string | null {
  const prefix = `--${name}=`;
  const hit = process.argv.find((a) => a.startsWith(prefix));
  return hit ? hit.slice(prefix.length).trim() : null;
}

function hasFlag(name: string): boolean {
  return process.argv.includes(`--${name}`);
}

function normalizePersonPart(s: string): string {
  return s
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

function personKey(nom: string, prenom: string): string {
  return `${normalizePersonPart(nom)}§${normalizePersonPart(prenom)}`;
}

function isBlank(v: string | null | undefined): boolean {
  return !String(v ?? "").trim();
}

type Patch = {
  ine?: string;
  nom: string;
  prenom: string;
  dateNaissance?: string;
  lieuNaissance?: string;
  sexe?: "M" | "F";
  regime?: string;
  classe?: string;
  mef?: string;
  email?: string;
  parentEmail?: string;
  parent1Email?: string;
  parent2Email?: string;
  parentPhone?: string;
  parent1Phone?: string;
  parent2Phone?: string;
};

function toPatch(e: EleveConfig): Patch {
  return {
    ine: e.ine,
    nom: e.nom,
    prenom: e.prenom,
    dateNaissance: normalizeEleveDateNaissance(e.dateNaissance ?? "") || undefined,
    lieuNaissance: e.lieuNaissance,
    sexe: e.sexe,
    regime: e.regime,
    classe: e.classe,
    mef: e.mef,
    email: e.email,
    parentEmail: e.parentEmail,
    parent1Email: e.parent1Email,
    parent2Email: e.parent2Email,
    parentPhone: e.parentPhone,
    parent1Phone: e.parent1Phone,
    parent2Phone: e.parent2Phone,
  };
}

function findMatch(
  existing: EleveConfig[],
  incoming: Patch,
): { eleve: EleveConfig; how: "ine" | "person" } | null {
  const ine = incoming.ine?.trim().toUpperCase();
  if (ine) {
    const byIne = existing.find((e) => e.ine?.trim().toUpperCase() === ine);
    if (byIne) return { eleve: byIne, how: "ine" };
  }
  const pk = personKey(incoming.nom, incoming.prenom);
  const candidates = existing.filter((e) => personKey(e.nom, e.prenom) === pk);
  if (candidates.length === 1) return { eleve: candidates[0], how: "person" };
  if (candidates.length > 1 && incoming.classe?.trim()) {
    const wanted = incoming.classe.trim().toUpperCase();
    const sameClass = candidates.filter(
      (e) => (e.classe?.trim().toUpperCase() || "") === wanted,
    );
    if (sameClass.length === 1) return { eleve: sameClass[0], how: "person" };
  }
  return null;
}

type FillPlan = {
  id: string;
  fields: Record<string, string>;
};

function planFill(existing: EleveConfig, incoming: Patch): FillPlan | null {
  const fields: Record<string, string> = {};
  const consider = (col: string, current: string | null | undefined, next: string | undefined) => {
    const v = String(next ?? "").trim();
    if (!v) return;
    if (!isBlank(current)) return;
    fields[col] = v;
  };

  consider("date_naissance", existing.dateNaissance, incoming.dateNaissance);
  consider("lieu_naissance", existing.lieuNaissance, incoming.lieuNaissance);
  consider("sexe", existing.sexe, incoming.sexe);
  consider("regime", existing.regime, incoming.regime);
  consider("classe", existing.classe, incoming.classe);
  consider("mef", existing.mef, incoming.mef);
  consider("email", existing.email, incoming.email);
  consider("parent_email", existing.parentEmail, incoming.parentEmail);
  consider("parent1_email", existing.parent1Email, incoming.parent1Email);
  consider("parent2_email", existing.parent2Email, incoming.parent2Email);
  consider("parent_phone", existing.parentPhone, incoming.parentPhone);
  consider("parent1_phone", existing.parent1Phone, incoming.parent1Phone);
  consider("parent2_phone", existing.parent2Phone, incoming.parent2Phone);
  consider("ine", existing.ine, incoming.ine?.trim().toUpperCase());

  if (Object.keys(fields).length === 0 || !existing.id) return null;
  return { id: existing.id, fields };
}

async function main() {
  if (!isDatabaseConfigured()) {
    throw new Error("DATABASE_URL manquant");
  }
  process.env.ENT_CORE_DB = process.env.ENT_CORE_DB || "1";
  process.env.ENT_IMPORT_SCRIPT = "1";
  process.env.NODE_TLS_REJECT_UNAUTHORIZED =
    process.env.NODE_TLS_REJECT_UNAUTHORIZED || "0";

  const filePath = argValue("file");
  const payloadPath = argValue("payload");
  const dryRun = hasFlag("dry-run");
  const slug =
    argValue("tenant") ||
    process.env.DEFAULT_TENANT_SLUG?.trim() ||
    "la-providence-nicolas-barre";

  let patches: Patch[] = [];

  if (payloadPath) {
    if (!existsSync(payloadPath)) throw new Error(`Payload introuvable : ${payloadPath}`);
    const raw = JSON.parse(readFileSync(payloadPath, "utf8")) as { patches?: Patch[] };
    patches = raw.patches ?? [];
  } else if (filePath) {
    if (!existsSync(filePath)) throw new Error(`Fichier introuvable : ${filePath}`);
    const buf = readFileSync(filePath);
    const parsed = parseElevesExcelBuffer(
      buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength),
      "auto",
    );
    if (!parsed.ok) throw new Error(parsed.error);
    patches = parsed.eleves.map(toPatch);
  } else {
    throw new Error('Usage: --file="..." ou --payload="..." [--tenant=...] [--dry-run]');
  }

  console.log(
    JSON.stringify(
      {
        phase: "parsed",
        tenant: slug,
        dryRun,
        patches: patches.length,
        withIne: patches.filter((p) => p.ine?.trim()).length,
        withDob: patches.filter((p) => p.dateNaissance).length,
        sample: patches.slice(0, 3),
      },
      null,
      2,
    ),
  );

  const db = getDb();
  const [etab] = await db
    .select()
    .from(etablissement)
    .where(eq(etablissement.slug, slug))
    .limit(1);
  if (!etab) {
    throw new Error(`Établissement introuvable pour le slug « ${slug} ».`);
  }

  const existing = await listElevesFromDb(etab.id);
  const beforeDob = existing.filter((e) => normalizeEleveDateNaissance(e.dateNaissance ?? "")).length;

  const plans: FillPlan[] = [];
  const stats = {
    matchedIne: 0,
    matchedPerson: 0,
    unmatched: 0,
    enriched: 0,
    alreadyComplete: 0,
    fieldsFilled: {} as Record<string, number>,
    unmatchedSample: [] as Array<{ ine: string; nom: string; prenom: string }>,
  };

  for (const inc of patches) {
    const match = findMatch(existing, inc);
    if (!match) {
      stats.unmatched += 1;
      if (stats.unmatchedSample.length < 20) {
        stats.unmatchedSample.push({
          ine: inc.ine || "",
          nom: inc.nom,
          prenom: inc.prenom,
        });
      }
      continue;
    }
    if (match.how === "ine") stats.matchedIne += 1;
    else stats.matchedPerson += 1;

    const plan = planFill(match.eleve, inc);
    if (!plan) {
      stats.alreadyComplete += 1;
      continue;
    }
    for (const f of Object.keys(plan.fields)) {
      stats.fieldsFilled[f] = (stats.fieldsFilled[f] || 0) + 1;
    }
    stats.enriched += 1;
    plans.push(plan);
  }

  console.log(
    JSON.stringify(
      {
        phase: "plan",
        etablissementId: etab.id,
        existingCount: existing.length,
        beforeWithDob: beforeDob,
        ...stats,
        toUpdate: plans.length,
      },
      null,
      2,
    ),
  );

  if (dryRun) {
    console.log(JSON.stringify({ phase: "dry-run", applied: false }, null, 2));
    await closeDb();
    return;
  }

  if (plans.length === 0) {
    console.log(JSON.stringify({ phase: "done", applied: false, reason: "rien à enrichir" }, null, 2));
    await closeDb();
    return;
  }

  let updated = 0;
  for (const plan of plans) {
    const patch: Record<string, unknown> = { updatedAt: new Date() };
    if (plan.fields.date_naissance) patch.dateNaissance = plan.fields.date_naissance;
    if (plan.fields.lieu_naissance) patch.lieuNaissance = plan.fields.lieu_naissance;
    if (plan.fields.sexe) patch.sexe = plan.fields.sexe;
    if (plan.fields.regime) patch.regime = plan.fields.regime;
    if (plan.fields.classe) patch.classe = plan.fields.classe;
    if (plan.fields.mef) patch.mef = plan.fields.mef;
    if (plan.fields.email) patch.email = plan.fields.email;
    if (plan.fields.parent_email) patch.parentEmail = plan.fields.parent_email;
    if (plan.fields.parent1_email) patch.parent1Email = plan.fields.parent1_email;
    if (plan.fields.parent2_email) patch.parent2Email = plan.fields.parent2_email;
    if (plan.fields.parent_phone) patch.parentPhone = plan.fields.parent_phone;
    if (plan.fields.parent1_phone) patch.parent1Phone = plan.fields.parent1_phone;
    if (plan.fields.parent2_phone) patch.parent2Phone = plan.fields.parent2_phone;
    if (plan.fields.ine) patch.ine = plan.fields.ine;

    await db
      .update(eleve)
      .set(patch)
      .where(and(eq(eleve.id, plan.id), eq(eleve.etablissementId, etab.id)));
    updated += 1;
  }

  const afterRows = await db
    .select({ id: eleve.id, dateNaissance: eleve.dateNaissance })
    .from(eleve)
    .where(eq(eleve.etablissementId, etab.id));
  const afterWithDob = afterRows.filter((r) => r.dateNaissance != null).length;

  console.log(
    JSON.stringify(
      {
        phase: "done",
        applied: true,
        updated,
        afterCount: afterRows.length,
        afterWithDob,
        deltaDob: afterWithDob - beforeDob,
      },
      null,
      2,
    ),
  );

  await closeDb();
}

main().catch(async (err) => {
  console.error(err);
  await closeDb().catch(() => undefined);
  process.exit(1);
});
