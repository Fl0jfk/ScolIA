// Parcours navigateur A11d–f (rapport docs/socles/15_ABSENCES_A11_DEF.md §5). Prérequis : Playwright global
// (chemin à adapter), axe-core 4.10.2 (variable AXE_PATH, sinon ./node_modules), serveur de labo démarré
// (cargo leptos build --bin-features labo). Usage : node parcours.mjs <base> <dossier-captures>
import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';
const require = createRequire(import.meta.url);
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const AXE = fs.readFileSync(process.env.AXE_PATH ?? new URL('./node_modules/axe-core/axe.min.js', import.meta.url), 'utf8');

const base = process.argv[2] ?? 'http://127.0.0.1:3000';
const sortie = process.argv[3] ?? './captures';
fs.mkdirSync(sortie, { recursive: true });
const MDP = 'DevLocalPass1!';
const rapport = { etapes: [], axe: [], mobile: [], clavier: [], erreurs: [] };
const journal = (m) => { console.log(m); rapport.etapes.push(m); };
const cap = (page, nom, pleine = true) => page.screenshot({ path: path.join(sortie, nom), fullPage: pleine });

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

// --- Jeu de données : un établissement neuf, via l'API de labo ---
const etab = crypto.randomUUID();
async function compte(role) {
  const email = `e2e-${role}-${crypto.randomUUID().slice(0, 8)}@ent.local`;
  await api('/auth/register', { methode: 'POST', corps: { email, password: MDP, etablissement_id: etab, role } });
  const s = await api('/auth/login', { methode: 'POST', corps: { email, password: MDP, etablissement_id: etab } });
  return { email, jeton: s.token, user_id: s.user_id, role };
}
const dir = await compte('direction');
const prof = await compte('prof');
const prof2 = await compte('prof');
const cpe = await compte('cpe');
const annuaire = await api('/personnels/annuaire?limite=200', { jeton: dir.jeton });
const agentDe = (c) => annuaire.find((a) => a.user_id === c.user_id).agent_id;

// Mois terminé précédent : l'état ne se fige qu'après son dernier jour.
const auj = new Date();
const premierMoisCourant = new Date(Date.UTC(auj.getUTCFullYear(), auj.getUTCMonth(), 1));
const leFait = new Date(premierMoisCourant.getTime() - 10 * 86400000).toISOString().slice(0, 10);
const heure = await api('/personnels/heures', { methode: 'POST', jeton: dir.jeton, corps: { agent_id: agentDe(prof), nature: 'hse', duree_minutes: 90, le: leFait } });
journal(`heure saisie par la direction (${heure.employeur}), le ${leFait}`);

const navigateur = await chromium.launch();

async function ouvrir(c, largeur = 1280, hauteur = 900) {
  const ctx = await navigateur.newContext({ viewport: { width: largeur, height: hauteur }, locale: 'fr-FR', deviceScaleFactor: 1, acceptDownloads: true });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => rapport.erreurs.push(`pageerror ${e.message}`));
  page.on('console', (m) => { if (m.type() === 'error') rapport.erreurs.push(`console ${m.text()}`); });
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
  await cap(page, `${nom}-375.png`);
  journal(`375 px ${nom} : débordement ${debord}px`);
  await ctx.close();
}

async function clavier(page, nom, attendus) {
  const vus = new Set();
  for (let i = 0; i < 90 && vus.size < attendus.length; i++) {
    await page.keyboard.press('Tab');
    const n = await page.evaluate(() => {
      const e = document.activeElement; if (!e) return '';
      const lib = e.labels && e.labels[0] ? e.labels[0].innerText : '';
      return (e.getAttribute('aria-label') || lib || e.innerText || e.getAttribute('placeholder') || e.id || e.tagName).trim().slice(0, 100);
    });
    for (const a of attendus) if (n.includes(a)) vus.add(a);
  }
  const manquants = attendus.filter((a) => !vus.has(a));
  rapport.clavier.push({ page: nom, atteints: [...vus], manquants });
  journal(`clavier ${nom} : ${manquants.length === 0 ? 'tout atteint' : 'manque ' + manquants.join(', ')}`);
}

async function boite(page, id) { await page.waitForSelector(`#${id}[open]`); }

// ===== A11d.1 Le professeur : mes heures, contestation =====
{
  const { ctx, page } = await ouvrir(prof);
  await page.goto(`${base}/web/personnel/mes-heures`);
  await page.waitForSelector('.absp-carte', { timeout: 20000 });
  await cap(page, '01-mes-heures.png');
  await axe(page, 'mes heures');
  await clavier(page, 'mes heures', ['Contester l\'heure']);
  await page.getByRole('button', { name: /Contester l'heure/ }).first().click();
  await boite(page, 'absp-dlg-contester');
  await cap(page, '02-contester-dialogue.png', false);
  await axe(page, 'mes heures : contester (dialogue)');
  await page.selectOption('#absp-contester-raison', 'duree_erronee');
  await page.fill('#absp-contester-note', 'Je crois que c\'était 1 h');
  await page.locator('#absp-dlg-contester').getByRole('button', { name: 'Contester', exact: true }).click();
  await page.waitForSelector('text=Contestée');
  await cap(page, '03-heure-contestee.png');
  await ctx.close();
}

// ===== A11d.2 La direction : régler, saisir, figer, signer, exporter, régulariser =====
{
  const { ctx, page } = await ouvrir(dir);
  await page.goto(`${base}/web/personnel/etats-heures`);
  await page.waitForSelector('.absp-carte', { timeout: 20000 });
  await cap(page, '04-heures-du-mois.png');
  await axe(page, 'heures du mois');
  await clavier(page, 'heures du mois', ['Régler la contestation', 'Saisir une heure sans séance']);
  // Correction qui règle la contestation.
  await page.getByRole('button', { name: /Corriger l'heure/ }).first().click();
  await boite(page, 'absp-corr-dlg');
  await page.fill('#absp-corr-duree', '1h');
  await cap(page, '05-corriger-dialogue.png', false);
  await axe(page, 'corriger (dialogue)');
  await page.locator('#absp-corr-dlg').getByRole('button', { name: 'Corriger', exact: true }).click();
  await page.waitForSelector('text=Validée');
  // Saisie d'une heure sans séance : le remplacement de courte durée n'est pas proposé (R-AP28).
  await page.getByRole('button', { name: 'Saisir une heure sans séance' }).click();
  await boite(page, 'absp-dlg-saisie');
  const natures = await page.locator('#absp-saisie-nature option').allInnerTexts();
  rapport.clavier.push({ page: 'saisie : natures proposées', natures });
  journal(`saisie : natures proposées ${natures.join(' | ')}`);
  if (natures.some((n) => /courte durée|Rattrapage/.test(n))) rapport.erreurs.push('une nature de remplacement est proposée à la saisie');
  await page.selectOption('#absp-saisie-agent', { label: (annuaire.find((a) => a.user_id === prof2.user_id)).nom_affiche });
  await page.fill('#absp-saisie-duree', '45 min');
  await page.fill('#absp-saisie-date', leFait);
  await cap(page, '06-saisir-dialogue.png', false);
  await axe(page, 'saisir une heure (dialogue)');
  await page.locator('#absp-dlg-saisie').getByRole('button', { name: 'Saisir', exact: true }).click();
  await page.waitForSelector('text=Heure saisie');
  // Valider les heures constatées du mois (celle de prof2).
  await page.getByRole('button', { name: /Valider les heures constatées/ }).click();
  await boite(page, 'absp-dlg-valider-heures');
  await cap(page, '07-valider-dialogue.png', false);
  await page.locator('#absp-dlg-valider-heures').getByRole('button', { name: 'Valider', exact: true }).click();
  await page.waitForSelector('text=heure validée');
  // État mensuel : figer, signer, exporter.
  await page.getByLabel('État mensuel').check();
  await page.waitForSelector('#absp-etat-aucun');
  await cap(page, '08-etat-a-figer.png');
  await axe(page, 'état mensuel (à figer)');
  await page.getByRole('button', { name: "Figer l'état du mois" }).click();
  await boite(page, 'absp-dlg-figer');
  await page.locator('#absp-dlg-figer').getByRole('button', { name: 'Figer', exact: true }).click();
  await page.waitForSelector('#absp-etat-titre');
  await page.waitForSelector('text=Figé, à signer');
  await cap(page, '09-etat-fige.png');
  await page.getByRole('button', { name: "Signer l'état" }).click();
  await boite(page, 'absp-dlg-signer');
  await page.locator('#absp-dlg-signer').getByRole('button', { name: 'Signer', exact: true }).click();
  await page.waitForSelector('text=Signé');
  await cap(page, '10-etat-signe.png');
  await axe(page, 'état mensuel (signé)');
  const [dl] = await Promise.all([page.waitForEvent('download', { timeout: 15000 }), page.getByRole('button', { name: 'Exporter (CSV)' }).click()]);
  const csv = fs.readFileSync(await dl.path(), 'utf8');
  journal(`export CSV : ${csv.split('\n').length} ligne(s), nom ${dl.suggestedFilename()}`);
  if (/maladie|formation|motif/i.test(csv)) rapport.erreurs.push('l\'export des heures parle d\'une absence');
  // Régularisation après figeage : jamais une réécriture.
  await page.getByLabel('Heures du mois').check();
  await page.waitForSelector('.absp-carte');
  await page.getByRole('button', { name: /Régulariser l'heure/ }).first().click();
  await boite(page, 'absp-regul-dlg');
  await page.fill('#absp-regul-duree', '45min');
  await cap(page, '11-regulariser-dialogue.png', false);
  await axe(page, 'régulariser (dialogue)');
  await page.locator('#absp-regul-dlg').getByRole('button', { name: 'Régulariser', exact: true }).click();
  await page.waitForSelector('text=Régularisation posée');
  await cap(page, '12-apres-regularisation.png');
  // Paie de l'OGEC et DSN, si la direction parcourt l'OGEC.
  if (await page.locator('#absp-employeur option[value=ogec]').count()) {
    await page.selectOption('#absp-employeur', 'ogec');
    await page.getByLabel('Paie de l\'OGEC').check();
    await page.waitForSelector('#absp-paie');
    await cap(page, '13-paie-ogec.png');
    await axe(page, 'paie de l\'OGEC');
    await page.getByLabel('DSN', { exact: true }).check();
    await page.waitForSelector('#absp-dsn');
    await cap(page, '14-dsn.png');
    await axe(page, 'DSN');
  }
  await ctx.close();
}

// ===== A11e. RCD : plan, volontaire, vivier, remontée =====
{
  const { ctx, page } = await ouvrir(dir);
  await page.goto(`${base}/web/personnel/rcd`);
  await page.waitForSelector('text=Pas encore de plan', { timeout: 20000 });
  await cap(page, '15-rcd-sans-plan.png');
  await page.getByRole('button', { name: "Créer le plan de l'année" }).click();
  await boite(page, 'absp-plan-creer-dlg');
  await page.fill('#absp-plan-creer-objectifs', 'Assurer la continuité des cours sans temps mort.');
  await page.fill('#absp-plan-creer-delai', '48');
  await cap(page, '16-plan-creer-dialogue.png', false);
  await axe(page, 'plan RCD : créer (dialogue)');
  await page.locator('#absp-plan-creer-dlg').getByRole('button', { name: 'Créer le plan', exact: true }).click();
  await page.waitForSelector('#absp-plan');
  await cap(page, '17-plan-brouillon.png');
  await axe(page, 'plan RCD (brouillon)');
  await page.getByRole('button', { name: 'Dates de procédure' }).click();
  await boite(page, 'absp-plan-procedure-dlg');
  const jourIso = new Date().toISOString().slice(0, 10);
  await page.fill('#absp-proc-avis', jourIso);
  await page.fill('#absp-proc-presente', jourIso);
  await page.selectOption('#absp-proc-instance', 'instance_etablissement');
  await page.locator('#absp-plan-procedure-dlg').getByRole('button', { name: 'Enregistrer', exact: true }).click();
  await page.waitForSelector('text=Présenté');
  await page.getByRole('button', { name: 'Lancer la campagne de volontariat' }).click();
  await boite(page, 'absp-dlg-campagne');
  await page.locator('#absp-dlg-campagne').getByRole('button', { name: 'Lancer la campagne', exact: true }).click();
  await page.waitForSelector('text=Campagne de volontariat lancée');
  await cap(page, '18-plan-presente.png');
  await ctx.close();
}
{
  // Le professeur se porte volontaire ; il ne voit ni le vivier ni la remontée.
  const { ctx, page } = await ouvrir(prof);
  await page.goto(`${base}/web/personnel/rcd`);
  await page.waitForSelector('#absp-plan', { timeout: 20000 });
  const sections = await page.locator('input[name=absp-section-rcd]').evaluateAll((l) => l.map((x) => x.value));
  rapport.clavier.push({ page: 'rcd (professeur) : sections', sections });
  journal(`rcd professeur : sections ${sections.join(', ')}`);
  if (sections.includes('vivier') || sections.includes('remontee')) rapport.erreurs.push('un professeur voit le vivier ou la remontée');
  await cap(page, '19-rcd-professeur.png');
  await page.getByLabel('Mon engagement').check();
  await page.waitForSelector('text=Vous n\'êtes pas volontaire');
  await page.getByRole('button', { name: 'Me porter volontaire' }).click();
  await boite(page, 'absp-dlg-engager-moi');
  await page.fill('#absp-dlg-engager-moi-volume', '18');
  await page.fill('#absp-dlg-engager-moi-du', new Date(Date.now() - 30 * 86400000).toISOString().slice(0, 10));
  await page.fill('#absp-dlg-engager-moi-au', new Date(Date.now() + 200 * 86400000).toISOString().slice(0, 10));
  await cap(page, '20-engagement-dialogue.png', false);
  await axe(page, 'engagement (dialogue)');
  await page.locator('#absp-dlg-engager-moi').getByRole('button', { name: 'Enregistrer', exact: true }).click();
  await page.waitForSelector('text=Mon engagement');
  await page.waitForSelector('text=médiane');
  await cap(page, '21-mon-compteur.png');
  await axe(page, 'mon compteur RCD');
  await ctx.close();
}
{
  const { ctx, page } = await ouvrir(dir);
  await page.goto(`${base}/web/personnel/rcd`);
  await page.waitForSelector('#absp-plan', { timeout: 20000 });
  await page.getByLabel('Volontaires').check();
  await page.waitForSelector('#absp-vivier');
  await page.waitForSelector('text=1 volontaire');
  await cap(page, '22-vivier.png');
  await axe(page, 'vivier RCD');
  await page.getByLabel('Remontée de la semaine').check();
  await page.waitForSelector('#absp-remontee');
  await cap(page, '23-remontee-vide.png');
  await page.getByRole('button', { name: 'Préparer la remontée' }).click();
  await page.waitForSelector('text=Préparée, à contrôler');
  await cap(page, '24-remontee-preparee.png');
  await axe(page, 'remontée RCD (préparée)');
  await page.getByRole('button', { name: 'Marquer contrôlée' }).click();
  await page.waitForSelector('text=Contrôlée');
  const [dl] = await Promise.all([page.waitForEvent('download', { timeout: 15000 }), page.getByRole('button', { name: 'Exporter (CSV)' }).click()]);
  const csv = fs.readFileSync(await dl.path(), 'utf8');
  journal(`export remontée : ${csv.split('\n').length} ligne(s)`);
  if (/@|agent|nom/i.test(csv)) rapport.erreurs.push('la remontée RCD porte un nom');
  await cap(page, '25-remontee-controlee.png');
  await page.getByRole('button', { name: 'Laisser partir' }).click();
  await boite(page, `absp-dlg-transmission-${await page.evaluate(() => document.querySelector('dialog[id^=absp-dlg-transmission-]').id.replace('absp-dlg-transmission-', ''))}`);
  await cap(page, '26-transmission-dialogue.png', false);
  await axe(page, 'remontée : laisser partir (dialogue)');
  await page.locator('dialog[open]').getByRole('button', { name: 'Laisser partir', exact: true }).click();
  await page.waitForSelector('text=Transmise');
  await cap(page, '27-remontee-transmise.png');
  await ctx.close();
}

// ===== A11f. Réglages =====
{
  const { ctx, page } = await ouvrir(dir);
  await page.goto(`${base}/web/personnel/parametrage-absences`);
  await page.waitForSelector('#absp-seuil-valeur', { timeout: 20000 });
  await cap(page, '28-reglages.png');
  await axe(page, 'réglages');
  await clavier(page, 'réglages', ['Seuil', 'Enregistrer le seuil']);
  // Hors bornes : refusé à l'écran, sans appel.
  await page.fill('#absp-seuil-valeur', '25');
  await page.getByRole('button', { name: 'Enregistrer le seuil' }).click();
  await page.waitForSelector('text=Le seuil va de 1 à 20');
  await cap(page, '29-seuil-hors-bornes.png', false);
  await page.fill('#absp-seuil-valeur', '5');
  await page.getByRole('button', { name: 'Enregistrer le seuil' }).click();
  await page.waitForSelector('text=Seuil réglé à 5');
  await page.waitForSelector('text=Réglé par l\'établissement');
  await cap(page, '30-seuil-regle.png');
  // Circuit : ajouter une règle.
  await page.getByRole('button', { name: 'Ajouter une règle' }).click();
  await boite(page, 'absp-dlg-regle');
  await page.selectOption('#absp-regle-agent', { label: (annuaire.find((a) => a.user_id === cpe.user_id)).nom_affiche });
  await page.selectOption('#absp-regle-role', 'informe');
  await cap(page, '31-regle-dialogue.png', false);
  await axe(page, 'ajouter une règle (dialogue)');
  await page.locator('#absp-dlg-regle').getByRole('button', { name: 'Ajouter', exact: true }).click();
  await page.waitForSelector('text=Règle ajoutée');
  await cap(page, '32-circuit.png');
  await page.getByRole('button', { name: /Allumer|Éteindre/ }).first().click().catch(() => {});
  await ctx.close();
}
{
  // Qui organise lit le seuil sans pouvoir le changer ; un professeur est refusé.
  const { ctx, page } = await ouvrir(cpe);
  await page.goto(`${base}/web/personnel/parametrage-absences`);
  await page.waitForSelector('#absp-seuil', { timeout: 20000 });
  await page.waitForSelector('text=Le seuil est réglé par la direction');
  if (await page.locator('#absp-seuil-valeur').count()) rapport.erreurs.push('le CPE peut modifier le seuil');
  await cap(page, '33-reglages-lecture-seule.png');
  await ctx.close();
}
{
  const { ctx, page } = await ouvrir(prof2);
  await page.goto(`${base}/web/personnel/parametrage-absences`);
  await page.waitForSelector('#absp-seuil', { timeout: 20000 });
  await page.waitForSelector('[role=alert], .ui-etat, .ui-alerte', { timeout: 15000 }).catch(() => rapport.erreurs.push('refus du professeur non affiché'));
  await cap(page, '34-reglages-professeur-refuse.png');
  const texte = await page.innerText('main');
  if (/Répond rarement/i.test(texte) && !/mention neutre|sans chiffre/i.test(texte)) rapport.erreurs.push('mention visible du professeur');
  await ctx.close();
}

// ===== 375 px =====
await mobile(prof, '/web/personnel/mes-heures', 'mes-heures');
await mobile(dir, '/web/personnel/etats-heures', 'etats-heures');
await mobile(dir, '/web/personnel/etats-heures', 'etat-mensuel', async (p) => { await p.getByLabel('État mensuel').check(); await p.waitForTimeout(1500); });
await mobile(dir, '/web/personnel/rcd', 'rcd-plan', async (p) => { await p.waitForSelector('#absp-plan'); });
await mobile(dir, '/web/personnel/rcd', 'rcd-remontee', async (p) => { await p.getByLabel('Remontée de la semaine').check(); await p.waitForSelector('#absp-remontee'); await p.waitForTimeout(1000); });
await mobile(dir, '/web/personnel/parametrage-absences', 'reglages', async (p) => { await p.waitForSelector('#absp-seuil-valeur'); });

await navigateur.close();
fs.writeFileSync(path.join(sortie, 'rapport-navigateur.json'), JSON.stringify(rapport, null, 2));
const graves = rapport.axe.flatMap((a) => a.violations.map((v) => ({ page: a.page, ...v })));
console.log(JSON.stringify({ violationsAxe: graves, mobile: rapport.mobile, clavier: rapport.clavier, erreurs: rapport.erreurs }, null, 2));
