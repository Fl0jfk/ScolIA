// Parcours navigateur des écrans du transverse individu (`docs/socles/TRANSVERSE_INDIVIDU.md` §10 et §11) :
// « Mes autres comptes » (tout compte) et « Fiches reliées » (direction, secrétariat), dans Chromium sans fenêtre,
// contre un serveur de labo déjà lancé (binaire construit par `cargo leptos build --bin-features labo`).
//
// Vérifie, à 375 px et à 1280 px : aucune violation axe-core (WCAG 2.1 A et AA), aucun défilement horizontal ;
// au clavier seulement (Tab, Entrée, Échap, flèches dans une liste) : confirmer une demande reçue, déclarer un
// autre compte, chercher et choisir deux fiches, relier avec un motif, lever ; le focus entre dans chaque boîte de
// dialogue et en ressort sur la page.
//
// Lancement (Playwright et axe-core installés à part, hors du dépôt) :
//   ENT_BASE=http://127.0.0.1:3000 AXE=/chemin/axe.min.js node ent-rust/web/tests-navigateur/individu.cjs
// Un établissement neuf par lancement ; données fictives ; aucun envoi réel.

const { chromium } = require('playwright');
const fs = require('fs');
const crypto = require('crypto');

const BASE = process.env.ENT_BASE || 'http://127.0.0.1:3000';
const AXE = fs.readFileSync(process.env.AXE || require.resolve('axe-core/axe.min.js'), 'utf8');
const MDP = 'DevLocalPass1!';
const ETAB = crypto.randomUUID();
const echecs = [];
const notes = [];

function verifier(ok, message) {
  if (!ok) echecs.push(message);
  else notes.push('ok  ' + message);
}

async function api(methode, chemin, corps, jeton) {
  const r = await fetch(BASE + chemin, {
    method: methode,
    headers: {
      'content-type': 'application/json',
      ...(jeton ? { authorization: 'Bearer ' + jeton, 'idempotency-key': crypto.randomUUID() } : {}),
    },
    body: corps ? JSON.stringify(corps) : undefined,
  });
  const texte = await r.text();
  if (!r.ok) throw new Error(`${methode} ${chemin} : ${r.status} ${texte}`);
  return texte ? JSON.parse(texte) : null;
}

async function compte(role) {
  const email = `nav-${role}-${crypto.randomUUID().slice(0, 8)}@ent.local`;
  await api('POST', '/auth/register', { email, password: MDP, etablissement_id: ETAB, role });
  const s = await api('POST', '/auth/login', { email, password: MDP, etablissement_id: ETAB });
  return { email, jeton: s.token };
}

/** Une fiche de responsable rattachée au compte (`PUT /dossier/personnes/{id}/compte`). */
async function ficheDuCompte(dir, c) {
  const me = await api('GET', '/auth/me', null, c.jeton);
  const p = await api('POST', '/dossier/personnes', { nom: 'Parcours', prenom: 'Navigateur' }, dir.jeton);
  await api('PUT', `/dossier/personnes/${p.id}/compte`, { user_id: me.user_id }, dir.jeton);
}

/** Connexion par l'écran (le cookie de session est posé par le serveur). */
async function connecter(page, email) {
  await page.goto(`${BASE}/web/connexion?etablissement_id=${ETAB}`);
  await page.waitForLoadState('networkidle');
  await page.locator('#email').fill(email);
  await page.locator('#mdp').fill(MDP);
  if (await page.locator('#etab').count()) await page.locator('#etab').fill(ETAB);
  await page.getByRole('button', { name: 'Continuer' }).click();
  await page.waitForURL((u) => !u.pathname.startsWith('/web/connexion'), { timeout: 15000 });
}

/** Attend l'hydratation (les boutons ne réagissent qu'une fois le WebAssembly chargé). */
async function hydrate(page) {
  await page.waitForLoadState('networkidle');
  await page.waitForFunction(() => document.querySelector('body') && !document.querySelector('.ui-squelette'), null, { timeout: 15000 });
}

async function audit(page, nom) {
  await page.addScriptTag({ content: AXE });
  const r = await page.evaluate(async () =>
    // eslint-disable-next-line no-undef
    axe.run(document, { runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'] } }));
  const v = r.violations.map((x) => `${x.id} (${x.impact}) : ${x.nodes.map((n) => n.target.join(' ')).slice(0, 3).join(' | ')}`);
  verifier(v.length === 0, `${nom} : axe-core sans violation${v.length ? '\n      ' + v.join('\n      ') : ''}`);
  const largeur = await page.evaluate(() => [document.documentElement.scrollWidth, window.innerWidth]);
  verifier(largeur[0] <= largeur[1], `${nom} : pas de défilement horizontal (${largeur[0]} ≤ ${largeur[1]})`);
}

/** Tab jusqu'à l'élément dont le nom accessible (ou le texte) vaut `nom`, 80 appuis au plus. */
async function tabJusqua(page, nom) {
  for (let i = 0; i < 80; i++) {
    await page.keyboard.press('Tab');
    const n = await page.evaluate(() => {
      const e = document.activeElement;
      return e ? (e.getAttribute('aria-label') || e.labels?.[0]?.textContent || e.textContent || '').trim() : '';
    });
    if (n === nom || n.startsWith(nom)) return true;
  }
  return false;
}

/** Boîte de dialogue ouverte et animation d'entrée finie (axe mesure le contraste pendant un fondu sinon). */
async function ouverte(page) {
  await page.locator('dialog[open]').waitFor();
  await page.waitForTimeout(800);
}

async function focusDans(page, selecteur) {
  return page.evaluate((s) => !!document.activeElement?.closest(s), selecteur);
}

async function mesAutresComptes(navigateur, largeur) {
  // La confirmation exige une fiche par compte : deux responsables rattachés à leur fiche par la direction.
  const dir = await compte('direction');
  const cible = await compte('parent');
  const declarant = await compte('parent');
  for (const c of [cible, declarant]) await ficheDuCompte(dir, c);
  // Le parent déclare le professeur (par l'API, comme le ferait son propre écran).
  await api('POST', '/dossier/individu/declarations', { email: cible.email }, declarant.jeton);

  const ctx = await navigateur.newContext({ viewport: { width: largeur, height: 900 }, bypassCSP: true, locale: 'fr-FR' });
  const page = await ctx.newPage();
  const erreursConsole = [];
  page.on('console', (m) => m.type() === 'error' && erreursConsole.push(m.text()));
  await connecter(page, cible.email);
  await page.goto(`${BASE}/web/securite/mes-comptes`);
  await hydrate(page);
  await page.getByRole('heading', { name: 'Demandes à confirmer' }).waitFor();
  await audit(page, `Mes autres comptes ${largeur} px (demande reçue)`);

  // Clavier : « C'est moi » → la boîte s'ouvre, focus sur « Annuler » ; Échap la ferme ; on recommence et on confirme.
  verifier(await tabJusqua(page, "C'est moi : le compte"), `Mes autres comptes ${largeur} px : « C'est moi » atteint au clavier`);
  await page.keyboard.press('Enter');
  await ouverte(page);
  verifier(await focusDans(page, '[role=dialog], dialog'), `Mes autres comptes ${largeur} px : le focus entre dans la confirmation`);
  await audit(page, `Mes autres comptes ${largeur} px (confirmation ouverte)`);
  await page.keyboard.press('Escape');
  await page.getByRole('dialog').waitFor({ state: 'hidden' });
  const retour = await page.evaluate(() => (document.activeElement?.getAttribute('aria-label') || document.activeElement?.tagName || ''));
  verifier(retour.startsWith("C'est moi : le compte"), `Mes autres comptes ${largeur} px : Échap ferme et rend le focus au bouton (focus sur « ${retour} »)`);
  await page.keyboard.press('Enter');
  await ouverte(page);
  verifier(await tabJusqua(page, "Oui, c'est mon compte"), `Mes autres comptes ${largeur} px : bouton de confirmation atteint au clavier`);
  await page.keyboard.press('Enter');
  await page.getByText('Confirmé : les deux comptes sont reconnus comme une seule personne.').first().waitFor({ timeout: 10000 });
  await page.getByText('1 autre compte de', { exact: false }).first().waitFor({ timeout: 10000 });
  verifier(true, `Mes autres comptes ${largeur} px : confirmation annoncée, lien affiché`);

  // Déclarer au clavier : Tab jusqu'au champ, saisie d'une adresse fausse puis juste, Entrée.
  verifier(await tabJusqua(page, "Adresse e-mail de l'autre compte"), `Mes autres comptes ${largeur} px : champ de déclaration atteint au clavier`);
  await page.keyboard.type('pas-une-adresse');
  await page.keyboard.press('Enter');
  await page.getByText("Cette adresse e-mail n'est pas valide.").first().waitFor();
  await page.keyboard.type('@nulle-part.example');
  await page.keyboard.press('Enter');
  await page.getByText('Demande enregistrée.', { exact: false }).first().waitFor({ timeout: 10000 });
  verifier((await page.locator('#ind-email').inputValue()) === '', `Mes autres comptes ${largeur} px : saisie effacée après l'accord du serveur`);
  await page.waitForTimeout(800); // animation d'entrée de la notification
  await audit(page, `Mes autres comptes ${largeur} px (après déclaration)`);
  verifier(erreursConsole.length === 0, `Mes autres comptes ${largeur} px : aucune erreur dans la console${erreursConsole.length ? ' : ' + erreursConsole.join(' | ') : ''}`);
  await ctx.close();
}

async function fichesReliees(navigateur, largeur) {
  const dir = await compte('direction');
  const nom = 'Navigo' + crypto.randomUUID().slice(0, 6);
  for (let i = 0; i < 2; i++) await api('POST', '/dossier/personnes', { nom, prenom: 'Individu' }, dir.jeton);

  const ctx = await navigateur.newContext({ viewport: { width: largeur, height: 900 }, bypassCSP: true, locale: 'fr-FR' });
  const page = await ctx.newPage();
  const erreursConsole = [];
  page.on('console', (m) => m.type() === 'error' && erreursConsole.push(m.text()));
  await connecter(page, dir.email);
  await page.goto(`${BASE}/web/direction/fiches-reliees`);
  await hydrate(page);
  await page.getByRole('heading', { name: 'Relier deux fiches' }).waitFor();
  await audit(page, `Fiches reliées ${largeur} px`);

  // Première fiche : chercher, choisir au clavier.
  for (const [n, rang] of [['Première fiche', 0], ['Seconde fiche', 1]]) {
    verifier(await tabJusqua(page, 'Nom, prénom, e-mail ou téléphone'), `Fiches reliées ${largeur} px : recherche « ${n} » atteinte au clavier`);
    await page.keyboard.type(nom);
    await page.keyboard.press('Enter');
    await page.getByText(/fiches? trouvées?/).first().waitFor({ timeout: 10000 });
    if (rang === 0) await audit(page, `Fiches reliées ${largeur} px (résultats de recherche)`);
    verifier(await tabJusqua(page, 'Choisir'), `Fiches reliées ${largeur} px : « Choisir » atteint au clavier (${n})`);
    await page.keyboard.press('Enter');
    await page.getByRole('button', { name: 'Changer' }).nth(rang).waitFor();
    await page.waitForTimeout(300); // le focus est posé juste après le rendu
    const f = await page.evaluate(() => document.activeElement?.textContent?.trim() || document.activeElement?.tagName || '');
    verifier(f === 'Changer', `Fiches reliées ${largeur} px : après « Choisir » (${n}), le focus va sur « Changer » (${f})`);
  }

  // Relier : la boîte de dialogue, motif au clavier (flèches dans la liste), confirmer.
  verifier(await tabJusqua(page, 'Relier ces deux fiches'), `Fiches reliées ${largeur} px : « Relier ces deux fiches… » atteint au clavier`);
  await page.keyboard.press('Enter');
  await ouverte(page);
  verifier(await focusDans(page, '[role=dialog], dialog'), `Fiches reliées ${largeur} px : le focus entre dans « Relier deux fiches »`);
  await audit(page, `Fiches reliées ${largeur} px (boîte « Relier »)`);
  verifier(await tabJusqua(page, 'Motif'), `Fiches reliées ${largeur} px : motif atteint au clavier`);
  await page.locator('#ind-motif-lien').selectOption('fiche_sans_compte');
  verifier(await tabJusqua(page, 'Relier'), `Fiches reliées ${largeur} px : bouton « Relier » atteint au clavier`);
  await page.keyboard.press('Enter');
  await page.getByText('Fiches reliées : les comptes concernés sont prévenus.').first().waitFor({ timeout: 10000 });
  await page.getByRole('button', { name: new RegExp('^Lever le lien entre .*' + nom) }).first().waitFor();
  await audit(page, `Fiches reliées ${largeur} px (lien actif, tableau)`);

  // Lever au clavier.
  verifier(await tabJusqua(page, 'Lever le lien entre ' + nom), `Fiches reliées ${largeur} px : « Lever… » atteint au clavier`);
  await page.keyboard.press('Enter');
  await ouverte(page);
  await page.locator('#ind-motif-levee').selectOption('erreur');
  verifier(await tabJusqua(page, 'Lever'), `Fiches reliées ${largeur} px : bouton « Lever » atteint au clavier`);
  await page.keyboard.press('Enter');
  await page.getByRole('button', { name: new RegExp('^Lever le lien entre .*' + nom) }).first().waitFor({ state: 'detached', timeout: 10000 });
  verifier(true, `Fiches reliées ${largeur} px : lien levé`);
  verifier(erreursConsole.length === 0, `Fiches reliées ${largeur} px : aucune erreur dans la console${erreursConsole.length ? ' : ' + erreursConsole.join(' | ') : ''}`);
  await ctx.close();
}

(async () => {
  const navigateur = await chromium.launch();
  try {
    for (const largeur of [375, 1280]) {
      for (const [nom, f] of [['Mes autres comptes', mesAutresComptes], ['Fiches reliées', fichesReliees]]) {
        try {
          await f(navigateur, largeur);
        } catch (e) {
          echecs.push(`${nom} ${largeur} px : parcours interrompu : ${e.message.split('\n').slice(0, 3).join(' ')}`);
        }
      }
    }
  } finally {
    await navigateur.close();
  }
  console.log(notes.join('\n'));
  if (echecs.length) {
    console.log('\nÉCHECS (' + echecs.length + ') :\n  ' + echecs.join('\n  '));
    process.exit(1);
  }
  console.log('\nParcours navigateur : tout est vert.');
})();
