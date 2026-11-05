import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

/** Règle projet : uniquement des polices présentes dans le dépôt (public/fonts). */
const INTERDIT = [/next\/font\/google/, /fonts\.googleapis\.com/, /fonts\.gstatic\.com/];

function* fichiers(dir: string): Generator<string> {
  for (const nom of readdirSync(dir)) {
    if (nom === "node_modules" || nom.startsWith(".")) continue;
    const chemin = join(dir, nom);
    if (statSync(chemin).isDirectory()) yield* fichiers(chemin);
    else if (/\.(tsx?|jsx?|mjs|css|scss)$/.test(nom) && !nom.endsWith("fonts-local-only.test.ts")) yield chemin;
  }
}

test("aucune police chargée depuis Google", () => {
  const fautifs: string[] = [];
  for (const racine of ["app", "public"]) {
    for (const f of fichiers(racine)) {
      const contenu = readFileSync(f, "utf8");
      if (INTERDIT.some((re) => re.test(contenu))) fautifs.push(f);
    }
  }
  assert.deepEqual(fautifs, [], `Polices externes trouvées : ${fautifs.join(", ")}`);
});
