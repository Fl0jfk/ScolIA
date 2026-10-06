import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const ROOT = path.join(import.meta.dirname, "..", "..");

/**
 * Fichiers « listes » : chaque `.from(eleve)` ou `loadElevesRegistry()` doit
 * utiliser le scope actif / période, sauf mention explicite dans le snippet (eq(eleve.id)).
 */
const WATCHED_LIST_FILES = [
  "app/lib/notes-bulletins-db.ts",
  "app/lib/notes-saisie-db.ts",
  "app/lib/groupes-pedagogiques-db.ts",
  "app/lib/famille-messaging-db.ts",
  "app/lib/famille-auth.ts",
  "app/lib/eleve-auth.ts",
  "app/lib/pilotage-eleves.ts",
  "app/api/eleves/route.ts",
  "app/api/travels/confirm-eleves-list/route.ts",
  "app/api/travels/send-parents/route.ts",
  "app/lib/vs-absences-db.ts",
  "app/lib/eleve-dossier-prof.ts",
  "app/lib/passages-prevision-db.ts",
  "app/lib/passages-db.ts",
  "app/lib/passages-facturation-db.ts",
  "app/lib/infirmerie-passages-db.ts",
  "app/lib/vs-sanctions-db.ts",
  "app/lib/vs-carnet-db.ts",
  "app/lib/classe-site-mapping.ts",
  "app/lib/sante-extraits-db.ts",
  "app/lib/accueil-absences-search.ts",
  "app/api/vie-scolaire/presence-jour/route.ts",
  "app/api/nomenclature/classes/route.ts",
  "app/lib/class-allocation-publish.ts",
  "app/lib/brain-ai/tools/handlers/eleves.ts",
  "app/api/toolbox/class-allocation/run/route.ts",
  "app/api/toolbox/class-allocation/rerun-level/route.ts",
  "app/api/toolbox/class-allocation/public/submit/route.ts",
  "app/api/toolbox/class-allocation/public/auth/request/route.ts",
  "app/api/toolbox/class-allocation/public/auth/verify/route.ts",
  "app/api/toolbox/class-allocation/public/auth/session/route.ts",
  "app/api/toolbox/class-allocation/staff/route.ts",
  "app/api/toolbox/class-allocation/classes/from-eleves/route.ts",
  "app/api/internat/students/roster/route.ts",
];

const ACTIF_SCOPE_MARKERS = [
  "drizzleEleveActifPourListes",
  "drizzleEleveVisiblePourPeriodeNotes",
  "eleveVisibilitePourNotes",
  "loadElevesActifsRegistry",
  "isEleveActifPourListes",
  "filterElevesScolarises",
];

function walkTsFiles(dir: string, out: string[] = []): string[] {
  for (const name of fs.readdirSync(dir)) {
    if (name === "node_modules" || name === ".next") continue;
    const full = path.join(dir, name);
    const st = fs.statSync(full);
    if (st.isDirectory()) {
      walkTsFiles(full, out);
    } else if (name.endsWith(".ts") && !name.endsWith(".test.ts")) {
      out.push(full);
    }
  }
  return out;
}

function snippetHasActifScope(snippet: string): boolean {
  if (snippet.includes("eq(eleve.id,") || snippet.includes("eq(eleve.id ,")) return true;
  return ACTIF_SCOPE_MARKERS.some((m) => snippet.includes(m));
}

function extractFromEleveSnippets(content: string): string[] {
  const snippets: string[] = [];
  let idx = 0;
  while (true) {
    const at = content.indexOf(".from(eleve)", idx);
    if (at < 0) break;
    const start = Math.max(0, at - 500);
    const end = Math.min(content.length, at + 1500);
    snippets.push(content.slice(start, end));
    idx = at + 12;
  }
  return snippets;
}

test("aucune requête liste ne filtre uniquement eq(eleve.status, inscrit)", () => {
  const offenders: string[] = [];
  const scanRoots = [path.join(ROOT, "app", "lib"), path.join(ROOT, "app", "api")];
  for (const root of scanRoots) {
    for (const file of walkTsFiles(root)) {
      if (file.includes("eleve-actif-scope.ts")) continue;
      const rel = path.relative(ROOT, file);
      const content = fs.readFileSync(file, "utf8");
      if (content.includes('eq(eleve.status, "inscrit")')) {
        offenders.push(rel);
      }
    }
  }
  assert.deepEqual(
    offenders,
    [],
    `Utiliser drizzleEleveActifPourListes() : ${offenders.join(", ")}`,
  );
});

test("fichiers listes — chaque .from(eleve) ou loadElevesRegistry() filtré (ou dossier par id)", () => {
  const offenders: string[] = [];

  for (const rel of WATCHED_LIST_FILES) {
    const full = path.join(ROOT, rel);
    if (!fs.existsSync(full)) {
      offenders.push(`${rel} (fichier manquant)`);
      continue;
    }
    const content = fs.readFileSync(full, "utf8");

    const usesFullRegistry =
      content.includes("loadElevesRegistry()") && !content.includes("loadElevesActifsRegistry()");
    const isImportHistorique =
      rel.includes("internat/students/roster") || rel.includes("nomenclature-import");
    if (usesFullRegistry && !isImportHistorique) {
      offenders.push(`${rel}: loadElevesRegistry() pour liste`);
    }

    for (const snippet of extractFromEleveSnippets(content)) {
      if (!snippetHasActifScope(snippet)) {
        offenders.push(`${rel}: .from(eleve) sans scope actif/période`);
        break;
      }
    }
  }

  assert.deepEqual(offenders, [], offenders.join("\n"));
});
