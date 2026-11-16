// Parcours navigateur de référence (rapport docs/socles/15_ABSENCES_A11_ABC.md §5). Prérequis : Playwright global
// (chemin ci-dessous à adapter), `npm install axe-core@4.10.2` à côté de ce fichier, serveur de labo démarré.
// Parcours navigateur A11a–c : déclarer → valider → ce matin → solliciter → accepter.
// Usage : node parcours.mjs <base> <dossier-captures>
import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';
const require = createRequire(import.meta.url);
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const AXE = fs.readFileSync(new URL('./node_modules/axe-core/axe.min.js', import.meta.url), 'utf8');

const base = process.argv[2] ?? 'http://127.0.0.1:8090';
const sortie = process.argv[3] ?? './captures';
fs.mkdirSync(sortie, { recursive: true });
const MDP = 'DevLocalPass1!';
const rapport = { etapes: [], axe: [], mobile: [], clavier: [], erreurs: [] };
const journal = (m) => { console.log(m); rapport.etapes.push(m); };

async function api(chemin, { methode = 'GET', jeton, corps } = {}) {
  const r = await fetch(base + chemin, {
    method: methode,
    headers: { 'content-type': 'application/json', ...(jeton ? { authorization: `Bearer ${jeton}` } : {}) },
    body: corps ? JSON.stringify(corps) : undefined,
  });
  const t = await r.text();
  let j = null; try { j = JSON.parse(t); } catch { /* texte */ }
  if (!r.ok) throw new Error(`${methode} ${chemin} → ${r.status} ${t.slice(0, 200)}`);
  return j;
}

// --- Jeu de données : un établissement neuf, via l'API (comptes de labo) ---
const etab = crypto.randomUUID();
async function compte(role) {
  const email = `e2e-${role}-${crypto.randomUUID().slice(0, 8)}@ent.local`;
  await api('/auth/register', { methode: 'POST', corps: { email, password: MDP, etablissement_id: etab, role } });
  const s = await api('/auth/login', { methode: 'POST', corps: { email, password: MDP, etablissement_id: etab } });
  return { email, jeton: s.token, user_id: s.user_id };
}
const dir = await compte('direction');
const prof = await compte('prof');
const prof2 = await compte('prof');

// Prochain jour ouvré à partir de demain.
function jourOuvre() {
  const d = new Date(Date.now() + 86400000);
  while ([0, 6].includes(d.getUTCDay())) d.setUTCDate(d.getUTCDate() + 1);
  return d.toISOString().slice(0, 10);
}
const jour = jourOuvre();
const iso = ((new Date(jour + 'T12:00:00Z').getUTCDay() + 6) % 7) + 1; // lundi = 1
const csv = [
  'classe;matiere;prof_user_id;jour_semaine;heure_debut;heure_fin;salle;semaine_ab',
  `4B;Mathématiques;${prof.user_id};${iso};09:00;10:00;12;toutes`,
  `4C;Mathématiques;${prof.user_id};${iso};10:00;11:00;12;toutes`,
  `6A;Français;${prof2.user_id};${iso};14:00;15:00;8;toutes`,
].join('\n');
try {
  await api('/edt/import', { methode: 'POST', jeton: dir.jeton, corps: { csv, dry_run: false } });
  journal(`EDT importé pour le ${jour}`);
  const annees = await api('/referentiel/annees-statut', { jeton: dir.jeton });
  const annee = (annees.find?.((x) => x.courante) ?? annees[0]).id;
  await api(`/referentiel/annees/${annee}/seances`, { methode: 'POST', jeton: dir.jeton, corps: { du: jour, au: jour } });
  journal('séances générées');
} catch (e) { journal(`EDT non importé : ${e.message}`); rapport.erreurs.push(e.message); }

const navigateur = await chromium.launch();

async function ouvrir(c, largeur = 1280) {
  const ctx = await navigateur.newContext({ viewport: { width: largeur, height: 900 }, locale: 'fr-FR', deviceScaleFactor: 1 });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => rapport.erreurs.push(`pageerror ${e.message}`));
  page.on('console', (m) => { if (m.type() === 'error') rapport.erreurs.push(`console ${m.text()}`); });
  page.on('response', (r) => { if (r.status() === 404) rapport.erreurs.push(`404 ${r.url()}`); });
  await page.goto(`${base}/web/connexion?etab=${etab}`);
  await page.waitForLoadState('networkidle');
  await page.waitForSelector('#email');
  await page.fill('#email', c.email);
  await page.fill('#mdp', MDP);
  if (await page.locator('#etab').count()) await page.fill('#etab', etab);
  await page.locator('form button[type=submit]').last().click();
  await page.waitForURL((u) => !u.pathname.includes('connexion'), { timeout: 20000 });
  await page.waitForLoadState('networkidle');
  return { ctx, page };
}

async function axe(page, nom) {
  await page.evaluate(AXE);
  const r = await page.evaluate(async () => {
    const x = await window.axe.run(document, { runOnly: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'best-practice'] });
    return x.violations.map((v) => ({ id: v.id, impact: v.impact, noeuds: v.nodes.length, cible: v.nodes[0]?.target?.join(' '), html: v.nodes[0]?.html?.slice(0, 160), aide: v.help }));
  });
  rapport.axe.push({ page: nom, violations: r });
  journal(`axe ${nom} : ${r.length} violation(s) ${r.map((v) => v.id).join(',')}`);
  for (const v of r) console.log(`   ↳ ${v.id} (${v.impact}) ${v.cible} :: ${v.html}`);
}

async function mobile(c, chemin, nom, actions) {
  const { ctx, page } = await ouvrir(c, 375);
  await page.goto(base + chemin);
  await page.waitForLoadState('networkidle');
  if (actions) await actions(page);
  const debord = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  rapport.mobile.push({ page: nom, debordement_px: debord });
  await page.screenshot({ path: path.join(sortie, `${nom}-375.png`), fullPage: true });
  journal(`375 px ${nom} : débordement ${debord}px`);
  await ctx.close();
}

async function clavier(page, nom, attendus) {
  // Tab jusqu'à atteindre chaque élément attendu (nom accessible), sans souris.
  const vus = new Set();
  for (let i = 0; i < 80 && vus.size < attendus.length; i++) {
    await page.keyboard.press('Tab');
    const n = await page.evaluate(() => {
      const e = document.activeElement; if (!e) return '';
      const lib = e.labels && e.labels[0] ? e.labels[0].innerText : '';
      return (e.getAttribute('aria-label') || lib || e.innerText || e.getAttribute('placeholder') || e.id || e.tagName).trim().slice(0, 80);
    });
    for (const a of attendus) if (n.includes(a)) vus.add(a);
  }
  const manquants = attendus.filter((a) => !vus.has(a));
  rapport.clavier.push({ page: nom, atteints: [...vus], manquants });
  journal(`clavier ${nom} : ${manquants.length === 0 ? 'tout atteint' : 'manque ' + manquants.join(', ')}`);
}

// ===== 1. Le professeur déclare son absence (A11a) =====
{
  const { ctx, page } = await ouvrir(prof);
  await page.goto(`${base}/web/personnel/absences`);
  await page.waitForLoadState('networkidle');
  await page.screenshot({ path: path.join(sortie, '01-mes-absences-vide.png'), fullPage: true });
  await axe(page, 'mes-absences (vide)');

  await page.getByRole('link', { name: 'Déclarer une absence' }).first().click();
  await page.waitForSelector('#decl-nature');
  await page.waitForLoadState('networkidle');
  await page.getByLabel('À des dates précises').check();
  await page.fill('#decl-debut', jour);
  await page.selectOption('#decl-nature', 'formation');
  await page.fill('#decl-texte', 'Formation numérique au rectorat');
  await page.waitForSelector('.absp-resume');
  await page.screenshot({ path: path.join(sortie, '02-declarer.png'), fullPage: true });
  await axe(page, 'declarer');
  await clavier(page, 'declarer', ['Nature de l\'absence', 'Déclarer l\'absence']);
  await page.getByRole('button', { name: "Déclarer l'absence" }).click();
  await page.waitForSelector('#absp-confirme-titre');
  await page.screenshot({ path: path.join(sortie, '03-declaration-confirmee.png'), fullPage: true });
  await axe(page, 'declaration confirmée');
  await page.getByRole('link', { name: "Ouvrir la fiche de l'absence" }).click();
  await page.waitForSelector('#absp-fiche-titre');
  await page.waitForLoadState('networkidle');
  await page.screenshot({ path: path.join(sortie, '04-fiche-agent.png'), fullPage: true });
  await axe(page, 'fiche (agent)');
  await ctx.close();
}

// ===== 2. La direction instruit (A11b) =====
let urlFiche = '';
{
  const { ctx, page } = await ouvrir(dir);
  await page.goto(`${base}/web/personnel/instruction-absences`);
  await page.waitForLoadState('networkidle');
  await page.waitForSelector('.absp-carte', { timeout: 15000 });
  await page.screenshot({ path: path.join(sortie, '05-file-a-valider.png'), fullPage: true });
  await axe(page, 'file à valider');
  await clavier(page, 'file', ['Ouvrir']);
  await page.getByRole('link', { name: /Ouvrir l'absence de/ }).first().click();
  await page.waitForSelector('#absp-fiche-titre');
  await page.waitForLoadState('networkidle');
  urlFiche = page.url();
  await page.screenshot({ path: path.join(sortie, '06-fiche-valideur.png'), fullPage: true });
  await axe(page, 'fiche (valideur)');
  // Clavier seul : Entrée ouvre la boîte, Échap la ferme, le focus revient sur « Valider ».
  const boutonValider = page.getByRole('button', { name: 'Valider', exact: true });
  await boutonValider.focus();
  await page.keyboard.press('Enter');
  await page.waitForSelector('#absp-dlg-valider[open]');
  const focusDansBoite = await page.evaluate(() => !!document.activeElement.closest('#absp-dlg-valider'));
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => !document.querySelector('#absp-dlg-valider').open);
  const retourFocus = await page.evaluate(() => document.activeElement.innerText.trim());
  rapport.clavier.push({ page: 'fiche (valideur) : boîte Valider', focusDansBoite, retourFocus });
  journal(`clavier boîte Valider : focus dans la boîte ${focusDansBoite}, retour du focus sur « ${retourFocus} »`);
  await boutonValider.click();
  await page.waitForSelector('#absp-dlg-valider[open]');
  await page.screenshot({ path: path.join(sortie, '07-dialogue-valider.png') });
  await axe(page, 'dialogue valider');
  await page.locator('#absp-dlg-valider').getByRole('button', { name: 'Valider', exact: true }).click();
  await page.waitForSelector('text=Validée');
  await page.screenshot({ path: path.join(sortie, '08-fiche-validee.png'), fullPage: true });
  // Onglets : séances touchées, historique.
  await page.getByLabel('Séances touchées').check();
  await page.waitForSelector('#absp-impact');
  await page.waitForLoadState('networkidle');
  await page.screenshot({ path: path.join(sortie, '09-onglet-seances.png'), fullPage: true });
  await page.getByLabel('Historique').check();
  await page.waitForSelector('#absp-historique');
  await page.screenshot({ path: path.join(sortie, '10-onglet-historique.png'), fullPage: true });
  await ctx.close();
}

// ===== 3. Ce matin : propositions et sollicitation (A11c) =====
{
  const { ctx, page } = await ouvrir(dir);
  await page.goto(`${base}/web/personnel/ce-matin`);
  await page.waitForLoadState('networkidle');
  // Le tableau d'aujourd'hui se charge seul, sans clic (régression : « Date invalide » affiché au premier passage).
  await page.waitForSelector('text=Séances à couvrir', { timeout: 20000 });
  journal('ce matin : chargement automatique du jour : ok');
  await page.getByLabel('Autre date').check();
  await page.fill('#absp-jour-date', jour);
  await page.waitForSelector('.absp-heure', { timeout: 15000 });
  await page.screenshot({ path: path.join(sortie, '11-ce-matin.png'), fullPage: true });
  await axe(page, 'ce matin');
  await clavier(page, 'ce matin', ['Répondre']);
  await page.getByRole('button', { name: /Répondre à la séance/ }).first().click();
  await page.waitForSelector('#absp-dlg-reponse[open]');
  await page.waitForTimeout(1200);
  await page.screenshot({ path: path.join(sortie, '12-propositions.png') });
  await axe(page, 'propositions (dialogue)');
  const solliciter = page.locator('#absp-dlg-reponse').getByRole('button', { name: /Demander à/ }).first();
  if (await solliciter.count()) {
    await solliciter.click();
    await page.waitForSelector('text=Demande envoyée');
    journal('sollicitation envoyée');
  } else {
    journal('aucun remplaçant proposé');
    rapport.erreurs.push('aucune proposition');
  }
  await page.waitForTimeout(800);
  await page.screenshot({ path: path.join(sortie, '13-apres-sollicitation.png'), fullPage: true });
  await ctx.close();
}

// ===== 4. Le remplaçant répond =====
{
  const { ctx, page } = await ouvrir(prof2);
  await page.goto(`${base}/web/personnel/remplacements`);
  await page.waitForLoadState('networkidle');
  await page.waitForSelector('.absp-carte', { timeout: 15000 }).catch(() => rapport.erreurs.push('aucune demande reçue'));
  await page.screenshot({ path: path.join(sortie, '14-mes-remplacements.png'), fullPage: true });
  await axe(page, 'mes remplacements');
  const accepter = page.getByRole('button', { name: /Accepter/ }).first();
  if (await accepter.count()) {
    await accepter.click();
    await page.waitForSelector('text=Remplacement accepté');
    await page.screenshot({ path: path.join(sortie, '15-remplacement-accepte.png'), fullPage: true });
  }
  await ctx.close();
}

// ===== 5. Permissions vues par le navigateur =====
{
  const { ctx, page } = await ouvrir(prof2);
  await page.goto(urlFiche);
  await page.waitForLoadState('networkidle');
  await page.waitForSelector('text=Absence introuvable', { timeout: 15000 }).catch(() => rapport.erreurs.push('fiche introuvable non affichée'));
  await page.screenshot({ path: path.join(sortie, '16-collegue-fiche-introuvable.png') });
  await page.goto(`${base}/web/personnel/ce-matin`);
  await page.waitForLoadState('networkidle');
  await page.waitForSelector('[role=alert], .ui-etat, .ui-alerte', { timeout: 15000 }).catch(() => rapport.erreurs.push('refus ce matin non affiché'));
  await page.screenshot({ path: path.join(sortie, '17-collegue-ce-matin-refuse.png') });
  await ctx.close();
}

// ===== 6. Largeur 375 px =====
await mobile(prof, '/web/personnel/absences', 'mes-absences');
await mobile(prof, '/web/personnel/absences/declarer', 'declarer');
await mobile(dir, '/web/personnel/instruction-absences', 'files');
await mobile(dir, urlFiche.replace(base, ''), 'fiche');
await mobile(dir, '/web/personnel/ce-matin', 'ce-matin', async (page) => {
  await page.getByLabel('Autre date').check();
  await page.fill('#absp-jour-date', jour);
  await page.waitForTimeout(1500);
});
await mobile(prof2, '/web/personnel/remplacements', 'mes-remplacements');

await navigateur.close();
fs.writeFileSync(path.join(sortie, 'rapport.json'), JSON.stringify(rapport, null, 2));
const graves = rapport.axe.flatMap((a) => a.violations.map((v) => ({ page: a.page, ...v })));
console.log(JSON.stringify({ violationsAxe: graves, mobile: rapport.mobile, clavier: rapport.clavier, erreurs: rapport.erreurs }, null, 2));
