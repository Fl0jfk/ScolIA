import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

/**
 * Règle projet : une seule police, Inter, servie depuis le dépôt (public/fonts).
 * Aucun service de polices externe.
 */
const INTERDIT = [/next\/font\/google/, /fonts\.googleapis\.com/, /fonts\.gstatic\.com/, /fonts\.bunny\.net/, /use\.typekit\.net/];

function* fichiers(dir: string, filtre: RegExp): Generator<string> {
  for (const nom of readdirSync(dir)) {
    if (nom === "node_modules" || nom.startsWith(".")) continue;
    const chemin = join(dir, nom);
    if (statSync(chemin).isDirectory()) yield* fichiers(chemin, filtre);
    else if (filtre.test(nom)) yield chemin;
  }
}

test("aucune police chargée depuis un service externe", () => {
  const fautifs: string[] = [];
  for (const racine of ["app", "public"]) {
    for (const f of fichiers(racine, /\.(tsx?|jsx?|mjs|css|scss)$/)) {
      if (f.endsWith("fonts-local-only.test.ts")) continue;
      if (INTERDIT.some((re) => re.test(readFileSync(f, "utf8")))) fautifs.push(f);
    }
  }
  assert.deepEqual(fautifs, [], `Polices externes trouvées : ${fautifs.join(", ")}`);
});

test("seule la police Inter est embarquée dans le projet", () => {
  const polices = ["app", "public"].flatMap((r) => [...fichiers(r, /\.(woff2?|ttf|otf|eot)$/i)]);
  const autres = polices.filter((f) => !/(^|[\\/])public[\\/]fonts[\\/]inter-latin(-italic)?\.woff2$/.test(f));
  assert.deepEqual(autres, [], `Autres polices trouvées : ${autres.join(", ")}`);
  assert.equal(polices.length, 2, "Inter normal + italique attendus");
});
