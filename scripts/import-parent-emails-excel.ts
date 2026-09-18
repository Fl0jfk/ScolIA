/**
 * Import Excel « mails parents » (Charlemagne / export collège) :
 * colonnes Nom | Prénom | Email | Email Personnel Resp | Email Personnel Conjoint
 *
 * Plusieurs lignes même nom/prénom = plusieurs foyers → on **ajoute** sans écraser,
 * et on relie toutes les adresses au même élève.
 *
 * Usage :
 *   npx tsx --require ./scripts/stub-server-only.cjs scripts/import-parent-emails-excel.ts \
 *     --file="P:/FLORIAN/Classeur….xlsx" \
 *     --tenant=la-providence-nicolas-barre
 *   # écriture réelle :
 *   … --apply
 */
import { existsSync, readFileSync } from "node:fs";
import { and, eq, inArray } from "drizzle-orm";
import { identityKey, normalizePersonPart } from "../app/lib/eleve-photos-match";
import {
  isValidParentEmail,
  normalizeParentEmail,
} from "../app/lib/eleves-parent-emails";
import { listElevesFromDb } from "../app/lib/ent-core-db";
import {
  groupParentEmailLines,
  parseParentEmailsExcelBuffer,
} from "../app/lib/parent-emails-excel-import";
import { closeDb, getDb, isDatabaseConfigured } from "../db/index";
import {
  eleve,
  eleveFoyerLink,
  etablissement,
  foyer,
  foyerResponsable,
} from "../db/schema";

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

function argFlag(name: string): boolean {
  return process.argv.includes(`--${name}`);
}

function localPart(email: string): string {
  return email.split("@")[0]?.trim() || "Parent";
}

async function main() {
  const file = argValue("file");
  const slug = argValue("tenant") || "la-providence-nicolas-barre";
  const apply = argFlag("apply");

  if (!file) {
    console.error("Usage: --file=…xlsx [--tenant=slug] [--apply]");
    process.exit(1);
  }
  if (!existsSync(file)) {
    console.error("Fichier introuvable:", file);
    process.exit(1);
  }
  if (!isDatabaseConfigured()) {
    console.error("DATABASE_URL manquant");
    process.exit(1);
  }

  const buf = readFileSync(file);
  const lines = parseParentEmailsExcelBuffer(buf);
  const groups = groupParentEmailLines(lines);
  console.log(
    JSON.stringify(
      {
        lines: lines.length,
        students: groups.length,
        multiFoyer: groups.filter((g) => g.foyers.length > 1).length,
        apply,
        tenant: slug,
      },
      null,
      2,
    ),
  );

  const db = getDb();
  const [etab] = await db
    .select({ id: etablissement.id, name: etablissement.name })
    .from(etablissement)
    .where(eq(etablissement.slug, slug))
    .limit(1);
  if (!etab) {
    console.error("Établissement introuvable:", slug);
    process.exit(1);
  }

  const eleves = await listElevesFromDb(etab.id, {
    status: ["preinscrit", "inscrit"],
  });
  const byIdentity = new Map<string, typeof eleves>();
  for (const e of eleves) {
    if (!e.id) continue;
    const k = identityKey(e.nom, e.prenom);
    const list = byIdentity.get(k) || [];
    list.push(e);
    byIdentity.set(k, list);
  }

  let matched = 0;
  let ambiguous = 0;
  let missing = 0;
  let foyersCreated = 0;
  let emailsAdded = 0;
  let elevesPatched = 0;
  const missingSamples: string[] = [];
  const ambiguousSamples: string[] = [];

  for (const g of groups) {
    if (!g.allEmails.length) continue;
    const hits = byIdentity.get(g.key) || [];
    if (hits.length === 0) {
      missing += 1;
      if (missingSamples.length < 15) {
        missingSamples.push(`${g.nom} ${g.prenom}`);
      }
      continue;
    }
    if (hits.length > 1) {
      ambiguous += 1;
      if (ambiguousSamples.length < 10) {
        ambiguousSamples.push(
          `${g.nom} ${g.prenom} (${hits.length} élèves id=${hits.map((h) => h.id).join(",")})`,
        );
      }
      continue;
    }

    const target = hits[0]!;
    const eleveId = target.id!;
    matched += 1;

    const links = await db
      .select({ foyerId: eleveFoyerLink.foyerId })
      .from(eleveFoyerLink)
      .where(
        and(
          eq(eleveFoyerLink.etablissementId, etab.id),
          eq(eleveFoyerLink.eleveId, eleveId),
        ),
      );
    const foyerIds = links.map((l) => l.foyerId);
    const knownEmails = new Set<string>();
    if (foyerIds.length) {
      const resps = await db
        .select({ email: foyerResponsable.email, foyerId: foyerResponsable.foyerId })
        .from(foyerResponsable)
        .where(
          and(
            eq(foyerResponsable.etablissementId, etab.id),
            inArray(foyerResponsable.foyerId, foyerIds),
          ),
        );
      for (const r of resps) {
        const e = normalizeParentEmail(String(r.email || ""));
        if (isValidParentEmail(e)) knownEmails.add(e);
      }
    }

    const currentSlots = [
      normalizeParentEmail(String(target.parentEmail || "")),
      normalizeParentEmail(String(target.parent1Email || "")),
      normalizeParentEmail(String(target.parent2Email || "")),
    ];
    const slotSet = new Set(currentSlots.filter((e) => isValidParentEmail(e)));
    const toFill = g.allEmails.filter((e) => !slotSet.has(e));
    if (toFill.length) {
      let parentEmail = String(target.parentEmail || "").trim() || null;
      let parent1Email = String(target.parent1Email || "").trim() || null;
      let parent2Email = String(target.parent2Email || "").trim() || null;
      for (const e of toFill) {
        if (!parentEmail) parentEmail = e;
        else if (!parent1Email) parent1Email = e;
        else if (!parent2Email) parent2Email = e;
        else break;
      }
      if (apply) {
        await db
          .update(eleve)
          .set({
            parentEmail,
            parent1Email,
            parent2Email,
            updatedAt: new Date(),
          })
          .where(and(eq(eleve.etablissementId, etab.id), eq(eleve.id, eleveId)));
      }
      elevesPatched += 1;
    }

    for (const foyerEmails of g.foyers) {
      const missingOnEleve = foyerEmails.filter((e) => !knownEmails.has(e));
      if (missingOnEleve.length === 0) continue;

      if (apply) {
        const familyLabel =
          normalizePersonPart(g.nom).replace(/\s+/g, " ") || g.nom || "Famille";
        const [created] = await db
          .insert(foyer)
          .values({
            etablissementId: etab.id,
            label: `Foyer ${familyLabel}`.slice(0, 120),
            payeurEstFoyer: true,
          })
          .returning({ id: foyer.id });
        if (!created?.id) continue;

        let rang = 1;
        for (const email of foyerEmails) {
          await db.insert(foyerResponsable).values({
            etablissementId: etab.id,
            foyerId: created.id,
            nom: g.nom || familyLabel,
            prenom: localPart(email),
            email,
            telephone: null,
            autoriteParentale: true,
            contactUrgence: rang === 1,
            payeur: false,
            rang,
          });
          knownEmails.add(email);
          emailsAdded += 1;
          rang += 1;
        }

        await db
          .insert(eleveFoyerLink)
          .values({
            etablissementId: etab.id,
            eleveId,
            foyerId: created.id,
            relation: foyerIds.length === 0 ? "principal" : "secondaire",
          })
          .onConflictDoNothing();
        foyerIds.push(created.id);
        foyersCreated += 1;
      } else {
        foyersCreated += 1;
        emailsAdded += missingOnEleve.length;
        for (const e of missingOnEleve) knownEmails.add(e);
      }
    }
  }

  console.log(
    JSON.stringify(
      {
        matched,
        missing,
        ambiguous,
        elevesPatched,
        foyersCreated,
        emailsAdded,
        missingSamples,
        ambiguousSamples,
        note: apply
          ? "Écriture effectuée."
          : "Dry-run uniquement — relancer avec --apply pour écrire.",
      },
      null,
      2,
    ),
  );

  await closeDb();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
