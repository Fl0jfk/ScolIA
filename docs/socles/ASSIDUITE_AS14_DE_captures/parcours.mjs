// Parcours navigateur AS14d et AS14e (rapport docs/socles/ASSIDUITE_AS14_DE.md).
// Prérequis : Playwright global (chemin ci-dessous), `npm install axe-core@4.10.2` à côté de ce fichier,
// serveur de labo démarré (ENT_ENV=lab, feature labo). Usage : node parcours.mjs <base> <dossier-captures>
import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';
const require = createRequire(import.meta.url);
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const AXE = fs.readFileSync(new URL('./node_modules/axe-core/axe.min.js', import.meta.url), 'utf8');

const base = process.argv[2] ?? 'http://127.0.0.1:8090';
const sortie = process.argv[3] ?? './captures';
fs.mkdirSync(sortie, { recursive: true });
const rapport = { etapes: [], axe: [], mobile: [], clavier: [], erreurs: [], verifs: [] };
const journal = (m) => { console.log(m); rapport.etapes.push(m); };
const verifier = (ok, m) => { rapport.verifs.push({ ok, m }); console.log(`${ok ? 'OK ' : 'ÉCHEC'} ${m}`); };

async function api(chemin, { methode = 'GET', jeton, corps } = {}) {
  const r = await fetch(base + chemin, {
    method: methode,
    headers: { 'content-type': 'application/json', ...(jeton ? { authorization: `Bearer ${jeton}` } : {}) },
    body: corps ? JSON.stringify(corps) : undefined,
  });
  const t = await r.text();
  let j = null; try { j = JSON.parse(t); } catch { /* texte */ }
  if (!r.ok) { const e = new Error(`${methode} ${chemin} → ${r.status} ${t.slice(0, 200)}`); e.statut = r.status; throw e; }
  return j;
}

// --- Jeu de données : établissement de labo, comptes nommés, quatre absences non justifiées d'Hermione
//     le mois précédent (mois terminé), un justificatif médical de la famille de Harry.
const labo = await api('/labo/bootstrap', { methode: 'POST', corps: {} });
const MDP = labo.password;
const etab = labo.etablissement_id;
async function session(email) {
  const s = await api('/auth/login', { methode: 'POST', corps: { email, password: MDP, etablissement_id: etab } });
  return { email, jeton: s.token };
}
const parRole = (r) => labo.personnel.find((p) => p.role === r);
const prof = await session(parRole('prof').email);
const cpe = await session(parRole('cpe').email);
const direction = await session(parRole('direction').email);
const secretariat = await session(parRole('administratif').email);
const parentHarry = await session(labo.personas.find((p) => p.role === 'parent' && p.label.includes('Harry')).email);
const hermione = labo.eleves.find((e) => e.label?.includes('Hermione') || e.prenom === 'Hermione');
const harry = labo.eleves.find((e) => e.label?.includes('Harry') || e.prenom === 'Harry');

const auj = new Date();
const premier = new Date(Date.UTC(auj.getUTCFullYear(), auj.getUTCMonth() - 1, 1));
const jourSemaine = auj.getUTCDay();
const jours = [];
for (let d = new Date(premier); jours.length < 4 && d.getUTCMonth() === premier.getUTCMonth(); d.setUTCDate(d.getUTCDate() + 1)) {
  if (d.getUTCDay() === jourSemaine) { jours.push(d.toISOString().slice(0, 10)); d.setUTCDate(d.getUTCDate() + 6); }
}
const moisPrec = premier.toISOString().slice(0, 7);
async function appelFait(jour, eleves) {
  const cren = (await api(`/edt/mes-creneaux?jour=${jour}`, { jeton: prof.jeton })).find((c) => c.classe === '4B');
  const ap = await api('/appels', { methode: 'POST', jeton: prof.jeton, corps: { creneau_id: cren.id, jour } });
  const v2 = await api(`/appels/${ap.id}/v2`, { jeton: prof.jeton });
  if (v2.etat === 'en_cours') {
    for (const e of eleves) await api(`/appels/${ap.id}/v2/lignes`, { methode: 'POST', jeton: prof.jeton, corps: { eleve_id: e, etat: 'absent' } });
    await api(`/appels/${ap.id}/v2/valider`, { methode: 'POST', jeton: prof.jeton, corps: { confirme_non_pointes: false } });
  }
}
for (const j of jours) await appelFait(j, [hermione.id]);
await appelFait(jours[0], [hermione.id, harry.id]).catch(() => {});
journal(`données : absences d'Hermione les ${jours.join(', ')} (mois ${moisPrec})`);
const eps = await api(`/assiduite/episodes?eleve_id=${harry.id}&etat=tous`, { jeton: cpe.jeton });
if (eps.length) {
  await api(`/assiduite/episodes/${eps[0].id}/justificatifs`, { methode: 'POST', jeton: parentHarry.jeton, corps: { texte: 'Certificat du docteur Rogue, angine', medical: true, vu_non_conserve: false } }).catch((e) => journal(`justificatif médical : ${e.message}`));
}
await api('/vs/signalements/proposer', { methode: 'POST', jeton: cpe.jeton, corps: { mois: `${moisPrec}-01` } });

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
    return x.violations.map((v) => ({ id: v.id, impact: v.impact, noeuds: v.nodes.length, cible: v.nodes[0]?.target?.join(' '), html: v.nodes[0]?.html?.slice(0, 160) }));
  });
  rapport.axe.push({ page: nom, violations: r });
  journal(`axe ${nom} : ${r.length} violation(s) ${r.map((v) => v.id).join(',')}`);
  for (const v of r) console.log(`   ↳ ${v.id} (${v.impact}) ${v.cible} :: ${v.html}`);
}
const photo = (page, nom) => page.screenshot({ path: path.join(sortie, `${nom}.png`), fullPage: true });
async function attendre(page) { await page.waitForLoadState('networkidle'); await page.waitForTimeout(600); }

// ===== AS14d : le CPE ouvre le dossier d'Hermione par la recherche =====
{
  const { ctx, page } = await ouvrir(cpe);
  await page.goto(`${base}/web/assiduite`);
  await attendre(page);
  await page.getByText("Dossier d'un élève", { exact: true }).click();
  await page.fill('#dossier-recherche', 'Hermione');
  await page.keyboard.press('Enter');
  await page.getByRole('button', { name: /Ouvrir le dossier de/ }).first().click();
  await page.waitForURL(/assiduite\/eleves\//);
  await attendre(page);
  await photo(page, '01-dossier-resume');
  await axe(page, 'dossier résumé');
  verifier(await page.getByText('Dossier d\'assiduité : Hermione').count() > 0, 'le dossier porte le nom de l\'élève');

  // Section Absences : ouvrir un épisode, qualifier, requalifier.
  await page.getByText('Absences', { exact: true }).click();
  await attendre(page);
  await photo(page, '02-dossier-absences');
  await axe(page, 'dossier absences');
  await page.getByRole('button', { name: /Ouvrir l'absence/ }).first().click();
  await page.waitForSelector('.asd-detail');
  await attendre(page);
  if (await page.locator('#asd-qualif').count()) {
    await page.selectOption('#asd-qualif', 'sans_motif');
    await page.getByRole('button', { name: "Qualifier l'absence" }).click();
    await attendre(page);
  }
  await page.getByRole('button', { name: /Ouvrir l'absence/ }).first().click();
  await attendre(page);
  await photo(page, '03-dossier-episode-requalification');
  if (await page.locator('#req-qualif').count()) {
    await page.selectOption('#req-qualif', 'excuse_valable');
    await page.getByRole('button', { name: "Requalifier l'absence" }).click();
    await page.waitForSelector('#req-motif-erreur, [role=alert], .ui-alerte', { timeout: 5000 }).catch(() => {});
    await photo(page, '04-requalification-motif-manquant');
    await page.selectOption('#req-motif', 'nouvelle_piece');
    await page.getByRole('button', { name: "Requalifier l'absence" }).click();
    await attendre(page);
    verifier(true, 'requalification posée depuis le dossier (motif codé)');
  } else { verifier(false, 'formulaire de requalification absent'); }
  await axe(page, 'dossier épisode + requalification');

  // Décompte par mois, rectifications.
  await page.getByText('Décompte', { exact: true }).click();
  await attendre(page);
  await photo(page, '05-dossier-decompte');
  await axe(page, 'dossier décompte');
  verifier(await page.getByText('Compte, non justifiée').count() + await page.getByText('Compte, justifiée').count() > 0, 'décompte : lignes expliquées par demi-journée');

  await page.getByText('Contacts et courriers', { exact: true }).click();
  await attendre(page);
  await photo(page, '06-dossier-contacts');
  await axe(page, 'dossier contacts');
  await page.getByText('Chronologie', { exact: true }).click();
  await attendre(page);
  await photo(page, '07-dossier-chronologie');
  await axe(page, 'dossier chronologie');

  // Clavier : les cinq sections se parcourent aux flèches, sans souris.
  await page.goto(`${base}/web/assiduite/eleves/${hermione.id}`);
  await attendre(page);
  await page.locator('input[name=dossier-vue]').first().focus();
  await page.keyboard.press('ArrowRight');
  await page.keyboard.press('ArrowRight');
  await attendre(page);
  verifier(await page.getByRole('heading', { name: /Décompte par demi-journée/ }).count() > 0, 'clavier : flèches entre les sections du dossier');
  await ctx.close();
}

// ===== AS14e : signalements (CPE propose, direction décide) =====
{
  const { ctx, page } = await ouvrir(cpe);
  await page.goto(`${base}/web/signalements`);
  await attendre(page);
  await page.fill('#sig-mois', moisPrec);
  await attendre(page);
  await photo(page, '10-signalements-cpe');
  await axe(page, 'signalements (CPE)');
  verifier(await page.getByText('Au seuil').count() > 0, 'élèves au seuil affichés');
  await page.locator('summary', { hasText: 'Relire le dossier pré-rempli' }).first().click();
  await photo(page, '11-signalement-relecture');
  await axe(page, 'signalements (relecture ouverte)');
  verifier(await page.getByText("La décision revient à la direction").count() > 0 || await page.getByRole('button', { name: /^Valider/ }).count() === 0, 'le CPE ne décide pas');
  await page.getByRole('button', { name: 'Préparer les signalements du mois' }).click();
  await photo(page, '12-signalements-confirmation-proposition');
  await axe(page, 'signalements (confirmation)');
  await page.getByRole('button', { name: 'Annuler' }).click();
  await page.goto(`${base}/web/signalements`);
  await attendre(page);
  await page.fill('#sig-mois', moisPrec);
  await attendre(page);
  await page.locator('input#sig-mois').focus();
  await ctx.close();
}
{
  const { ctx, page } = await ouvrir(direction);
  await page.goto(`${base}/web/signalements`);
  await attendre(page);
  await page.fill('#sig-mois', moisPrec);
  await attendre(page);
  const valider = page.getByRole('button', { name: /^Valider/ }).first();
  verifier(await valider.count() > 0, 'la direction voit « Valider »');
  await valider.click();
  await photo(page, '13-signalements-direction-confirmer');
  await page.getByRole('button', { name: 'Confirmer' }).click();
  await attendre(page);
  await photo(page, '14-signalements-valide');
  verifier(await page.getByText(/Validé, à transmettre/).count() > 0, 'signalement validé, à transmettre');
  verifier(await page.getByText(/n'envoie rien à l'académie/).count() > 0, 'le texte dit que ScolIA n\'envoie rien');
  await axe(page, 'signalements (direction, validé)');
  await ctx.close();
}

// ===== Droits : secrétariat, motif médical =====
{
  const { ctx, page } = await ouvrir(secretariat);
  await page.goto(`${base}/web/assiduite/eleves/${harry.id}`);
  await attendre(page);
  await page.getByText('Absences', { exact: true }).click();
  await attendre(page);
  const ouvrirBtn = page.getByRole('button', { name: /Ouvrir l'absence/ }).first();
  if (await ouvrirBtn.count()) { await ouvrirBtn.click(); await attendre(page); }
  await photo(page, '20-secretariat-dossier-harry');
  verifier(await page.getByText('angine').count() === 0, 'secrétariat : aucun texte médical');
  await page.getByText('Contacts et courriers', { exact: true }).click();
  await attendre(page);
  verifier(await page.getByText('Contacts non accessibles').count() > 0, 'secrétariat : contacts dits non accessibles');
  await photo(page, '21-secretariat-contacts-refuses');
  await axe(page, 'secrétariat : contacts refusés');
  await ctx.close();
}

// ===== 375 px =====
async function mobile(c, chemin, nom, actions) {
  const { ctx, page } = await ouvrir(c, 375);
  await page.goto(base + chemin);
  await attendre(page);
  if (actions) await actions(page);
  const debord = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  rapport.mobile.push({ page: nom, debordement_px: debord });
  await photo(page, `${nom}-375`);
  await axe(page, `${nom} (375 px)`);
  journal(`375 px ${nom} : débordement ${debord}px`);
  await ctx.close();
}
await mobile(cpe, `/web/assiduite/eleves/${hermione.id}`, '30-dossier-resume');
await mobile(cpe, `/web/assiduite/eleves/${hermione.id}`, '31-dossier-decompte', async (p) => { await p.getByText('Décompte', { exact: true }).click(); await p.waitForTimeout(800); });
await mobile(cpe, `/web/assiduite/eleves/${hermione.id}`, '32-dossier-absences', async (p) => { await p.getByText('Absences', { exact: true }).click(); await p.waitForTimeout(800); });
await mobile(direction, '/web/signalements', '33-signalements', async (p) => { await p.fill('#sig-mois', moisPrec); await p.waitForTimeout(1000); });

await navigateur.close();
fs.writeFileSync(path.join(sortie, 'rapport.json'), JSON.stringify(rapport, null, 2));
const nbAxe = rapport.axe.reduce((n, a) => n + a.violations.length, 0);
console.log(`\nRésumé : ${rapport.axe.length} vues axe, ${nbAxe} violation(s) ; vérifications en échec : ${rapport.verifs.filter((v) => !v.ok).length} ; erreurs navigateur : ${rapport.erreurs.length}`);
if (rapport.erreurs.length) console.log(rapport.erreurs.slice(0, 10));
