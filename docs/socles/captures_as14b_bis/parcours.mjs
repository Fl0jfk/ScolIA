// Parcours navigateur du tableau de bord de la vie scolaire en panneaux (rapport docs/socles/ASSIDUITE_AS14B_BIS.md).
// Prérequis : Playwright global (chemin ci-dessous à adapter), `axe-core@4.10.2` installé hors du dépôt (variable AXE_MIN = chemin de axe.min.js),
// serveur `ent-web-server` compilé avec `labo` démarré sur une base LOCALE de test, base PostgreSQL accessible en `psql`.
// Usage : ENT_DATABASE_URL=postgres://… node parcours.mjs <base> <dossier-captures>
// Données : distribution de labo (POST /labo/bootstrap), actions par l'API ; les alertes « urgentes » sont posées par SQL
// dans la base LOCALE de test (la tâche système qui les crée tourne dans le binaire ent-api, pas dans ent-web-server).
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
const require = createRequire(import.meta.url);
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const AXE = fs.readFileSync(process.env.AXE_MIN ?? new URL('./node_modules/axe-core/axe.min.js', import.meta.url), 'utf8');

const base = process.argv[2] ?? 'http://127.0.0.1:8090';
const sortie = process.argv[3] ?? './captures';
const DB = process.env.ENT_DATABASE_URL;
if (!DB || !/127\.0\.0\.1|localhost/.test(DB) || !/test|proto|lab/.test(DB)) throw new Error('base locale de test uniquement');
fs.mkdirSync(sortie, { recursive: true });
const rapport = { etapes: [], axe: [], mobile: [], clavier: [], erreurs: [], controles: [] };
const journal = (m) => { console.log(m); rapport.etapes.push(m); };
const controle = (nom, ok, detail = '') => { rapport.controles.push({ nom, ok, detail }); console.log(`${ok ? 'OK ' : 'ÉCHEC'} ${nom} ${detail}`); if (!ok) rapport.erreurs.push(`contrôle : ${nom} ${detail}`); };
const sql = (q) => execFileSync('psql', [DB, '-v', 'ON_ERROR_STOP=1', '-At', '-c', q], { encoding: 'utf8' }).trim();

async function api(chemin, { methode = 'GET', jeton, corps } = {}) {
  const r = await fetch(base + chemin, { method: methode, headers: { 'content-type': 'application/json', ...(jeton ? { authorization: `Bearer ${jeton}` } : {}) }, body: corps ? JSON.stringify(corps) : undefined });
  const t = await r.text(); let j = null; try { j = JSON.parse(t); } catch { /* texte */ }
  if (!r.ok) throw new Error(`${methode} ${chemin} → ${r.status} ${t.slice(0, 200)}`);
  return j;
}

// --- Jeu de données : la distribution de labo (établissement fictif) ---
const labo = await api('/labo/bootstrap', { methode: 'POST', corps: {} });
const etab = labo.etablissement_id; const MDP = labo.password;
const pers = (role) => labo.personnel.find((p) => p.role === role);
const eleve = (prenom) => labo.eleves.find((e) => e.prenom === prenom);
async function jeton(email) { return (await api('/auth/login', { methode: 'POST', corps: { email, password: MDP, etablissement_id: etab } })).token; }
const cpe = pers('cpe'); const prof = pers('prof'); const dir = pers('direction');
const tCpe = await jeton(cpe.email); const tProf = await jeton(prof.email); const tDir = await jeton(dir.email);
const harry = eleve('Harry'); const hermione = eleve('Hermione'); const ginny = eleve('Ginny');
const aujourdhui = new Date(Date.now()).toISOString().slice(0, 10);
const jourParis = sql(`SELECT (now() AT TIME ZONE 'Europe/Paris')::date`);
journal(`établissement ${etab}, jour ${jourParis}`);

// Nettoyage rejouable : disposition de la CPE, heure de bascule, alertes du jour.
await api('/assiduite/tableau/preference', { methode: 'DELETE', jeton: tCpe });
await api('/assiduite/tableau/heure-bascule', { methode: 'PUT', jeton: tDir, corps: { heure: '11:00:00' } });
sql(`DELETE FROM ent_asd_alerte WHERE etablissement_id = '${etab}' AND jour = '${jourParis}'`);

// 1. L'appel de 4B (séance du jour de Rogue) : Harry et Hermione absents, Ginny exclue de cours.
const creneaux = await api(`/edt/mes-creneaux?jour=${jourParis}`, { jeton: tProf }).catch(() => null);
const c4b = (creneaux ?? []).find((c) => c.classe === '4B');
if (!c4b) throw new Error('séance 4B du jour introuvable');
const appel = await api('/appels', { methode: 'POST', jeton: tProf, corps: { creneau_id: c4b.id, jour: jourParis } });
await api(`/appels/${appel.id}/v2/lignes`, { methode: 'POST', jeton: tProf, corps: { eleve_id: harry.id, etat: 'absent' } });
await api(`/appels/${appel.id}/v2/lignes`, { methode: 'POST', jeton: tProf, corps: { eleve_id: hermione.id, etat: 'absent' } });
await api(`/appels/${appel.id}/exclusions`, { methode: 'POST', jeton: tProf, corps: { eleve_id: ginny.id } });
// Exclusion ancienne (25 min) : c'est elle qui est dans le bandeau Urgent ; l'alerte « non arrivé » est posée comme le fait la tâche.
sql(`UPDATE ent_appel_ligne SET exclusion_cours_le = now() - interval '25 minutes' WHERE etablissement_id = '${etab}' AND appel_id = '${appel.id}' AND eleve_id = '${ginny.id}'`);
sql(`INSERT INTO ent_asd_alerte (id, etablissement_id, eleve_id, kind, jour, appel_id, cle) VALUES (gen_random_uuid(), '${etab}', '${ginny.id}', 'non_arrivee', '${jourParis}', '${appel.id}', 'parcours-${Date.now()}')`);
await api(`/appels/${appel.id}/v2/valider`, { methode: 'POST', jeton: tProf, corps: { confirme_non_pointes: true } });
// Hermione : la famille a été appelée sans succès (« familles à rappeler »).
await api('/assiduite/contacts', { methode: 'POST', jeton: tCpe, corps: { eleve_id: hermione.id, canal: 'telephone', issue: 'ne_repond_pas' } });
journal(`appel ${appel.id} : Harry et Hermione absents, Ginny exclue (non arrivée)`);

const navigateur = await chromium.launch();
async function ouvrir(c, largeur = 1280) {
  const ctx = await navigateur.newContext({ viewport: { width: largeur, height: 900 }, locale: 'fr-FR', deviceScaleFactor: 1 });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => rapport.erreurs.push(`pageerror ${e.message}`));
  page.on('console', (m) => { if (m.type() === 'error') rapport.erreurs.push(`console ${m.text()}`); });
  await page.goto(`${base}/web/connexion?etab=${etab}`);
  await page.waitForLoadState('networkidle');
  await page.waitForSelector('#email');
  await page.fill('#email', c.email); await page.fill('#mdp', MDP);
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
const titres = (page) => page.$$eval('.tdb-grille .tdb-panneau__titre', (els) => els.map((e) => e.textContent.trim()));
async function attendreTableau(page) {
  await page.waitForSelector('#tdb-urgent-titre', { timeout: 30000 });
  await page.waitForLoadState('networkidle');
}
const clic = async (page, nom, opts = {}) => { await page.getByRole('button', { name: nom, ...opts }).first().click(); };

// ===== MATIN : 08 h 30 (avant la bascule) =====
{
  const { ctx, page } = await ouvrir(cpe);
  await page.goto(`${base}/web?heure=08:30`);
  await attendreTableau(page);
  const t = await titres(page);
  controle('matin : absents en premier', t[0] === 'Absents du jour à traiter', t.join(' | '));
  controle('matin : grand panneau pleine largeur', await page.$eval('.tdb-grille .tdb-panneau:first-child', (e) => e.classList.contains('tdb-panneau--grand')));
  const urgent = await page.$eval('.tdb-urgent', (e) => e.textContent);
  controle('bandeau Urgent affiche Ginny (exclue non arrivée)', /Ginny/.test(urgent) && /Exclu de cours, non arrivé/.test(urgent), urgent.slice(0, 120));
  controle('absents : Harry et Hermione', await page.locator('.tdb-panneau', { hasText: 'Absents du jour à traiter' }).getByText(/Harry|Hermione/).count() >= 1);
  await page.screenshot({ path: path.join(sortie, '01-matin.png'), fullPage: true });
  await axe(page, 'matin (08:30)');
  await ctx.close();
}

// ===== APRÈS-MIDI : 14 h 30 (après la bascule) =====
{
  const { ctx, page } = await ouvrir(cpe);
  await page.goto(`${base}/web?heure=14:30`);
  await attendreTableau(page);
  const t = await titres(page);
  controle('après-midi : exclus puis appels en premier', t[0] === 'Exclus de cours' && t[1] === 'Appels non faits', t.join(' | '));
  controle('après-midi : deux grands panneaux', (await page.$$eval('.tdb-panneau--grand', (e) => e.length)) === 2);
  await page.screenshot({ path: path.join(sortie, '02-apres-midi.png'), fullPage: true });
  await axe(page, 'après-midi (14:30)');

  // Action en ligne : « Je prends » sur l'alerte du bandeau, puis « Arrivé » dans le panneau des exclus.
  await clic(page, /Je prends : Ginny/);
  await page.waitForSelector('text=Pris en charge', { timeout: 15000 });
  controle('« Je prends » : verrou visible', true);
  await clic(page, /Confirmer l'arrivée de Ginny/);
  await page.waitForSelector('text=Aucun exclu de cours en attente d\'arrivée', { timeout: 15000 });
  controle('« Arrivé » : Ginny quitte le panneau des exclus', true);
  await page.screenshot({ path: path.join(sortie, '03-apres-actions.png'), fullPage: true });
  await ctx.close();
}

// ===== DISPOSITION : épingler, replier, masquer, retrouver sur « un autre poste », revenir à automatique =====
{
  const { ctx, page } = await ouvrir(cpe);
  await page.goto(`${base}/web?heure=08:30`);
  await attendreTableau(page);
  await page.getByRole('button', { name: 'Disposition', exact: true }).click();
  await page.waitForSelector('#tdb-dispo:not([hidden])');
  await page.screenshot({ path: path.join(sortie, '04-disposition.png'), fullPage: true });
  await axe(page, 'disposition ouverte');
  // Clavier : Tab jusqu'à « Monter Exclus de cours », Entrée.
  await page.getByRole('button', { name: 'Monter Exclus de cours' }).focus();
  await page.keyboard.press('Enter');
  await page.waitForSelector('text=Disposition enregistrée', { timeout: 15000 });
  await clic(page, 'Masquer Incohérences d\'appel');
  await page.waitForSelector('text=(masqué)', { timeout: 15000 });
  await clic(page, /Replier.*Familles à rappeler|Replier Familles/);
  await page.waitForTimeout(1500);
  await page.screenshot({ path: path.join(sortie, '05-epinglee.png'), fullPage: true });
  const t = await titres(page);
  controle('épinglé : Incohérences masqué', !t.includes("Incohérences d'appel"), t.join(' | '));
  await ctx.close();
  // « Autre poste » : nouveau contexte de navigateur, même compte, à une autre heure : l'ordre épinglé tient.
  const autre = await ouvrir(cpe);
  await autre.page.goto(`${base}/web?heure=14:30`);
  await attendreTableau(autre.page);
  const t2 = await titres(autre.page);
  controle('autre poste, après-midi : ordre épinglé retrouvé (pas l\'automatique de l\'après-midi)', t2[0] === 'Absents du jour à traiter' && t2[1] === 'Exclus de cours' && t2[2] === 'Familles à rappeler', t2.join(' | '));
  controle('le bandeau Urgent n\'a ni bouton replier ni masquer', (await autre.page.locator('.tdb-urgent button', { hasText: /Replier|Masquer/ }).count()) === 0);
  await autre.page.screenshot({ path: path.join(sortie, '06-autre-poste.png'), fullPage: true });
  await autre.page.getByRole('button', { name: 'Disposition', exact: true }).click();
  await clic(autre.page, 'Revenir à automatique');
  await autre.page.waitForSelector('text=Disposition automatique rétablie', { timeout: 15000 });
  await autre.page.waitForTimeout(800);
  const t3 = await titres(autre.page);
  controle('revenir à automatique : exclus et appels en premier l\'après-midi', t3[0] === 'Exclus de cours' && t3[1] === 'Appels non faits', t3.join(' | '));
  await autre.ctx.close();
}

// ===== Heure de bascule réglable par l'établissement (direction) =====
{
  const { ctx, page } = await ouvrir(dir);
  await page.goto(`${base}/web?heure=10:15`);
  await attendreTableau(page);
  let t = await titres(page);
  controle('bascule 11 h : à 10 h 15, absents en premier', t[0] === 'Absents du jour à traiter', t.join(' | '));
  await page.getByRole('button', { name: 'Disposition', exact: true }).click();
  await page.fill('#tdb-bascule', '10:00');
  await clic(page, "Enregistrer l'heure");
  await page.waitForSelector('text=Heure de bascule réglée à 10:00', { timeout: 15000 });
  t = await titres(page);
  controle('bascule réglée à 10 h : à 10 h 15, exclus en premier', t[0] === 'Exclus de cours', t.join(' | '));
  await page.screenshot({ path: path.join(sortie, '07-bascule-direction.png'), fullPage: true });
  await page.fill('#tdb-bascule', '11:00'); await clic(page, "Enregistrer l'heure");
  await page.waitForSelector('text=Heure de bascule réglée à 11:00', { timeout: 15000 });
  await ctx.close();
}

// ===== Prévenir maintenant (confirmation obligatoire) =====
{
  const { ctx, page } = await ouvrir(cpe);
  await page.goto(`${base}/web?heure=08:30`);
  await attendreTableau(page);
  await clic(page, /Prévenir maintenant la famille de Harry/);
  await page.waitForSelector('#tdb-dlg-prevenir[open]');
  const bouton = page.locator('#tdb-dlg-prevenir').getByRole('button', { name: "Envoyer l'avis urgent" });
  controle('prévenir maintenant : bouton inactif tant qu\'aucune raison n\'est choisie', await bouton.isDisabled());
  const focusDans = await page.evaluate(() => !!document.activeElement.closest('#tdb-dlg-prevenir'));
  await page.screenshot({ path: path.join(sortie, '08-dialogue-prevenir.png') });
  await axe(page, 'dialogue prévenir maintenant');
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => !document.querySelector('#tdb-dlg-prevenir').open);
  const retour = await page.evaluate(() => document.activeElement.getAttribute('aria-label') || document.activeElement.innerText);
  controle('dialogue : focus dedans à l\'ouverture, retour du focus sur le bouton à Échap', focusDans && /Prévenir maintenant/.test(retour), retour);
  await ctx.close();
}

// ===== Téléphone 375 px : panneaux empilés, Urgent en tête, panneaux repliables =====
{
  const { ctx, page } = await ouvrir(cpe, 375);
  await page.goto(`${base}/web?heure=08:30`);
  await attendreTableau(page);
  const debord = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  controle('375 px : aucun débordement horizontal', debord <= 0, `${debord}px`);
  const boites = await page.$$eval('.tdb-urgent, .tdb-grille .tdb-panneau', (els) => els.map((e) => { const r = e.getBoundingClientRect(); return { x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), classe: e.className }; }));
  controle('375 px : Urgent en tête puis panneaux empilés (même colonne)', boites[0].classe.includes('tdb-urgent') && boites.every((b, i) => i === 0 || (b.x === boites[1].x && b.y > boites[i - 1].y)), JSON.stringify(boites.slice(0, 3)));
  await page.screenshot({ path: path.join(sortie, '09-telephone.png'), fullPage: true });
  await axe(page, 'téléphone 375 px');
  // Replier un panneau au téléphone (mode automatique : local à la page).
  await clic(page, /Replier.*Absents du jour/);
  await page.waitForSelector('#tdb-p-absents-corps[hidden]', { state: 'attached' });
  controle('375 px : panneau repliable', true);
  await page.screenshot({ path: path.join(sortie, '10-telephone-replie.png'), fullPage: true });
  await ctx.close();
}

// ===== Bandeau Urgent vide : discret, « Rien d'urgent » =====
{
  sql(`DELETE FROM ent_asd_alerte WHERE etablissement_id = '${etab}' AND jour = '${jourParis}'`);
  const { ctx, page } = await ouvrir(cpe);
  await page.goto(`${base}/web?heure=14:30`);
  await attendreTableau(page);
  const ok = await page.locator('.tdb-urgent--vide', { hasText: "Rien d'urgent" }).count();
  controle('bandeau vide : « Rien d\'urgent »', ok === 1);
  await page.screenshot({ path: path.join(sortie, '11-urgent-vide.png'), fullPage: true });
  await ctx.close();
}

// ===== Permissions : un professeur n'a pas le tableau =====
{
  const { ctx, page } = await ouvrir(prof);
  await page.goto(`${base}/web`);
  await page.waitForLoadState('networkidle');
  await page.waitForTimeout(1500);
  controle('professeur : aucun tableau de vie scolaire', (await page.locator('#tdb').count()) === 0);
  await ctx.close();
}

await navigateur.close();
fs.writeFileSync(path.join(sortie, 'rapport-navigateur.json'), JSON.stringify(rapport, null, 2));
const ko = rapport.controles.filter((c) => !c.ok).length;
const violations = rapport.axe.reduce((n, a) => n + a.violations.length, 0);
console.log(`\nContrôles : ${rapport.controles.length - ko}/${rapport.controles.length} ; axe : ${violations} violation(s) sur ${rapport.axe.length} vues ; erreurs : ${rapport.erreurs.length}`);
process.exit(ko || violations ? 1 : 0);
