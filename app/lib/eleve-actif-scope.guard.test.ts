import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const ROOT = path.join(import.meta.dirname, "..", "..");

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
  "app/lib/cahier-texte-db.ts",
  "app/lib/brain-ai/tools/handlers/eleves.ts",
  "app/lib/brain-ai/tools/handlers/accueil-absences.ts",
  "app/lib/brain-ai/tools/handlers/eleve-grille-repas.ts",
  "app/lib/eleve-accompagnement-alerts.ts",
  "app/lib/fiches-dialogue-workflow.ts",
  "app/api/eleves/registry-ids/route.ts",
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

const REGISTRY_IMPORT_WHITELIST = [
  "app/api/internat/students/roster/route.ts",
  "app/api/eleves/import/route.ts",
];

const ACTIF_SCOPE_MARKERS = [
  "drizzleEleveActifPourListes",
  "drizzleEleveVisiblePourPeriodeNotes",
  "eleveVisibilitePourNotes",
  "loadElevesActifsRegistry",
  "isEleveActifPourListes",
  "filterElevesScolarises",
];

const ELEVE_QUERY_MARKERS = [".from(eleve)", ".innerJoin(eleve", ".leftJoin(eleve"];

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

function snippetHasActifScope(snippet: string, fileContent: string): boolean {
  if (snippet.includes("eq(eleve.id,") || snippet.includes("eq(eleve.id ,")) return true;
  if (snippet.includes("visibilite!") && fileContent.includes("eleveVisibilitePourNotes")) return true;
  return ACTIF_SCOPE_MARKERS.some((m) => snippet.includes(m));
}

function extractEleveQuerySnippets(content: string): string[] {
  const snippets: string[] = [];
  for (const marker of ELEVE_QUERY_MARKERS) {
    let idx = 0;
    while (true) {
      const at = content.indexOf(marker, idx);
      if (at < 0) break;
      const start = Math.max(0, at - 600);
      const end = Math.min(content.length, at + 2400);
      snippets.push(content.slice(start, end));
      idx = at + marker.length;
    }
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

test("fichiers listes — jointures eleve et loadElevesRegistry filtrés", () => {
  const offenders: string[] = [];

  for (const rel of WATCHED_LIST_FILES) {
    const full = path.join(ROOT, rel);
    if (!fs.existsSync(full)) {
      offenders.push(`${rel} (fichier manquant)`);
      continue;
    }
    const content = fs.readFileSync(full, "utf8");

    const isImportWhitelist = REGISTRY_IMPORT_WHITELIST.some((w) => rel.includes(w));
    if (content.includes("loadElevesRegistry()") && !isImportWhitelist) {
      if (!content.includes("loadElevesActifsRegistry()")) {
        offenders.push(`${rel}: loadElevesRegistry() liste`);
      }
    }

    for (const snippet of extractEleveQuerySnippets(content)) {
      if (!snippetHasActifScope(snippet, content)) {
        offenders.push(`${rel}: requête eleve sans scope actif/période`);
        break;
      }
    }
  }

  assert.deepEqual(offenders, [], offenders.join("\n"));
});

test("api/ — loadElevesRegistry réservé import / historique", () => {
  const offenders: string[] = [];
  const apiRoot = path.join(ROOT, "app", "api");
  for (const file of walkTsFiles(apiRoot)) {
    const rel = path.relative(ROOT, file).replace(/\\/g, "/");
    const content = fs.readFileSync(file, "utf8");
    if (!content.includes("loadElevesRegistry()")) continue;
    const allowed =
      rel.includes("eleves/import") ||
      rel.includes("internat/students/roster") ||
      rel.includes("stages/contacts-import") ||
      rel.includes("agentIAOCR");
    if (!allowed && !content.includes("loadElevesActifsRegistry()")) {
      offenders.push(rel);
    }
  }
  assert.deepEqual(offenders, [], offenders.join("\n"));
});
