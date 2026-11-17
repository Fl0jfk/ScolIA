// Parcours navigateur E9a/E9b (rapport docs/socles/02_ELEVE_E9_LIVRAISON.md §6). Prérequis : Playwright global,
// axe-core 4.10.2 (AXE_PATH), serveur de labo démarré (cargo leptos build --bin-features labo, binaire ent-web-server).
// Usage : node parcours.mjs <base> <dossier-captures>   — code de sortie 1 si une violation axe ou un échec.
import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_PATH ?? '/opt/node22/lib/node_modules/playwright');
const AXE = fs.readFileSync(process.env.AXE_PATH ?? new URL('./node_modules/axe-core/axe.min.js', import.meta.url), 'utf8');
const base = process.argv[2] ?? 'http://127.0.0.1:8090';
const sortie = process.argv[3] ?? './captures';
fs.mkdirSync(sortie, { recursive: true });
const MDP = 'DevLocalPass1!';
const rapport = { etapes: [], axe: [], mobile: [], clavier: [], erreurs: [], echecs: [] };
const journal = (m) => { console.log(m); rapport.etapes.push(m); };
const verifie = (ok, m) => { journal(`${ok ? 'OK ' : 'ÉCHEC'} ${m}`); if (!ok) rapport.echecs.push(m); };
const cap = (page, nom) => page.screenshot({ path: path.join(sortie, nom), fullPage: true });

async function api(chemin, { methode = 'GET', jeton, corps } = {}) {
  const r = await fetch(base + chemin, { method: methode, headers: { 'content-type': 'application/json', ...(jeton ? { authorization: `Bearer ${jeton}` } : {}) }, body: corps ? JSON.stringify(corps) : undefined });
  const t = await r.text(); let j = null; try { j = JSON.parse(t); } catch { /* texte */ }
  if (!r.ok) throw new Error(`${methode} ${chemin} → ${r.status} ${t.slice(0, 200)}`);
  return j;
}
const etab = crypto.randomUUID();
async function compte(role) {
  const email = `e9-${role}-${crypto.randomUUID().slice(0, 8)}@ent.local`;
  await api('/auth/register', { methode: 'POST', corps: { email, password: MDP, etablissement_id: etab, role } });
  const s = await api('/auth/login', { methode: 'POST', corps: { email, password: MDP, etablissement_id: etab } });
  return { email, jeton: s.token, role };
}
const secretariat = await compte('administratif');
const direction = await compte('direction');
const cpe = await compte('cpe');
const prof = await compte('prof');

// 62 élèves (deux pages), accents et homonymes ; un élève protégé.
const prenoms = ['Chloé', 'Léa', 'Noé', 'Zoé', 'Hélène', 'Yanis', 'Inès', 'Mathis', 'Lucas', 'Emma'];
const noms = ['Martin', 'Bernard', 'Dubois', 'Thomas', 'Robert', 'Richard', 'Petit', 'Durand'];
const classes = ['6A', '5B', '4C'];
let protege = null;
for (let i = 0; i < 62; i++) {
  const e = await api('/eleves', { methode: 'POST', jeton: secretariat.jeton, corps: { nom: i === 7 ? "Zyxwvutsr" : `${noms[i % noms.length]}${i}`, prenom: prenoms[i % prenoms.length], classe: classes[i % 3], date_naissance: '2012-03-14', sexe: i % 2 ? 'F' : 'M' } });
  if (i === 7) protege = e;
}
await api(`/eleves/${protege.id}/confidentialite`, { methode: 'POST', jeton: direction.jeton, corps: { motif: 'Mesure de protection (parcours)', mesures: [] } });
journal(`62 élèves créés, ${protege.prenom} ${protege.nom} protégé`);

const navigateur = await chromium.launch();
async function ouvrir(c, largeur = 1280, hauteur = 900) {
  const ctx = await navigateur.newContext({ viewport: { width: largeur, height: hauteur }, locale: 'fr-FR' });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => rapport.erreurs.push(`pageerror ${e.message}`));
  page.on('console', (m) => {
    if (m.type() !== 'error') return;
    const u = m.location().url ?? '';
    // Attendus : la photo absente (404 d'une <img>, repli sur les initiales) et la session pas encore ouverte (401 avant connexion).
    if (/\/eleves\/[0-9a-f-]{36}\/photo$/.test(u) || (/Unauthorized/.test(m.text()) && /\/auth\/me$/.test(u))) return;
    rapport.erreurs.push(`console ${m.text()} ${u}`);
  });
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
  verifie(r.length === 0, `axe ${nom} : ${r.length} violation(s) ${r.map((v) => v.id).join(',')}`);
  for (const v of r) console.log(`   ↳ ${v.id} (${v.impact}) ${v.cible} :: ${v.html}`);
}
const lignes = (page) => page.locator('.eleves-tableau tbody tr');

// ===== Secrétariat : liste =====
{
  const { ctx, page } = await ouvrir(secretariat);
  await page.goto(`${base}/web/eleves`);
  await page.waitForSelector('.eleves-tableau tbody tr', { timeout: 20000 });
  verifie((await lignes(page).count()) === 50, 'première page : 50 lignes');
  verifie(await page.getByText(/1 à 50 sur 61 élèves/).first().isVisible(), 'compteur « 1 à 50 sur 61 » (le protégé n\'est pas compté pour un tiers)');
  await cap(page, '01-liste-secretariat.png');
  await axe(page, 'liste');
  // Raccourci « / », puis recherche tolérante aux accents.
  await page.locator('h1').click();
  await page.keyboard.press('/');
  verifie(await page.evaluate(() => document.activeElement?.id) === 'eleves-recherche', 'la touche / met le focus sur la recherche');
  await page.keyboard.type('chloe');
  await page.waitForFunction(() => document.querySelectorAll('.eleves-tableau tbody tr').length < 50, null, { timeout: 8000 });
  const noms1 = await lignes(page).allInnerTexts();
  verifie(noms1.length > 0 && noms1.every((t) => /Chlo/i.test(t)), `« chloe » ne trouve que des Chloé (${noms1.length})`);
  // « / » dans une saisie ne vole pas le caractère.
  await page.keyboard.type('/');
  verifie((await page.inputValue('#eleves-recherche')) === 'chloe/', 'la touche / reste un caractère pendant la saisie');
  await page.fill('#eleves-recherche', '');
  await page.waitForFunction(() => document.querySelectorAll('.eleves-tableau tbody tr').length === 50, null, { timeout: 8000 });
  // Filtre de classe.
  await page.selectOption('#eleves-classe', '6A');
  await page.waitForFunction(() => [...document.querySelectorAll('.eleves-tableau tbody tr td[data-label="Classe"]')].every((c) => c.textContent.trim() === '6A'), null, { timeout: 8000 });
  verifie(true, 'filtre classe 6A : toutes les lignes en 6A');
  await page.getByRole('button', { name: 'Effacer les filtres' }).click();
  await page.waitForFunction(() => document.querySelectorAll('.eleves-tableau tbody tr').length === 50);
  // Pagination.
  await page.getByRole('button', { name: 'Page suivante' }).click();
  await page.waitForFunction(() => /51 à 61 sur 61/.test(document.body.innerText), null, { timeout: 8000 });
  verifie((await lignes(page).count()) === 11, 'page 2 : 11 lignes');
  await page.getByRole('button', { name: 'Page précédente' }).click();
  await page.waitForFunction(() => /1 à 50 sur 61/.test(document.body.innerText), null, { timeout: 8000 });
  // Aucun résultat.
  await page.fill('#eleves-recherche', 'zzzzqqq');
  await page.getByText('Aucun élève ne correspond').waitFor({ timeout: 8000 });
  verifie(true, 'liste vide expliquée');
  await axe(page, 'liste vide');
  await page.fill('#eleves-recherche', '');
  await page.waitForSelector('.eleves-tableau tbody tr');
  // Clavier : recherche, classe, statut, premier élève, pagination.
  await page.locator('h1').click();
  const vus = new Set();
  for (let i = 0; i < 40 && vus.size < 5; i++) {
    await page.keyboard.press('Tab');
    const n = await page.evaluate(() => { const e = document.activeElement; const l = e?.labels?.[0]?.innerText ?? ''; return (e?.getAttribute('aria-label') || l || e?.innerText || e?.id || '').trim().slice(0, 60); });
    for (const a of ['Rechercher un élève', 'Classe', 'Statut']) if (n.includes(a)) vus.add(a);
    if (/^[A-ZÉ][\p{L}-]+ [A-ZÉ-]+\d+$/u.test(n)) vus.add('lien élève');
    if (n.includes('Page suivante')) vus.add('Page suivante');
  }
  rapport.clavier.push({ page: 'liste', atteints: [...vus] });
  verifie(['Rechercher un élève', 'Classe', 'Statut', 'lien élève'].every((a) => vus.has(a)), `clavier liste : ${[...vus].join(', ')}`);
  // Ouvrir la fiche au clavier (Entrée sur le lien).
  await page.locator('.eleves-nom a').first().focus();
  await page.keyboard.press('Enter');
  await page.waitForURL(/\/web\/eleves\/[0-9a-f-]{36}$/);
  await page.locator('.fiche-entete__nom').waitFor();
  const nom = await page.locator('.fiche-entete__nom').innerText();
  verifie(nom.length > 3, `fiche ouverte : ${nom}`);
  await page.waitForSelector('#tuile-identite-titre');
  await cap(page, '02-fiche-synthese.png');
  await axe(page, 'fiche synthèse');
  const tuiles = await page.locator('.ui-tuile__titre').allInnerTexts();
  verifie(tuiles.includes('Identité') && tuiles.includes('Santé'), `tuiles : ${tuiles.join(', ')}`);
  verifie(await page.getByText('Masqué').first().isVisible(), 'une tuile masquée dit « Masqué » et pourquoi');
  // Onglets : identité, historique (flèches du contrôle segmenté).
  await page.getByRole('radio', { name: 'Identité' }).check({ force: true });
  await page.getByText('État civil et scolarité').waitFor();
  verifie(page.url().includes('onglet=identite'), 'l\'onglet est dans l\'adresse');
  await axe(page, 'fiche identité');
  await page.getByRole('radio', { name: 'Historique' }).check({ force: true });
  await page.getByText('Historique de l\'identité').waitFor();
  await page.waitForSelector('.fiche-frise__etape');
  await cap(page, '03-fiche-historique.png');
  await axe(page, 'fiche historique');
  await ctx.close();
}

// ===== Élève protégé : absent pour la vie scolaire, présent pour la direction =====
{
  const { ctx, page } = await ouvrir(cpe);
  await page.goto(`${base}/web/eleves`);
  await page.waitForSelector('.eleves-tableau tbody tr');
  await page.fill('#eleves-recherche', protege.nom);
  await page.getByText('Aucun élève ne correspond').waitFor({ timeout: 8000 });
  verifie(true, 'le protégé n\'est pas dans la liste de la vie scolaire');
  await page.goto(`${base}/web/eleves/${protege.id}`);
  await page.getByText('Fiche introuvable').waitFor({ timeout: 15000 });
  verifie(true, 'sa fiche répond « introuvable », jamais « refusé »');
  await cap(page, '04-protege-vie-scolaire.png');
  await axe(page, 'fiche introuvable');
  await ctx.close();
  const d = await ouvrir(direction);
  await d.page.goto(`${base}/web/eleves/${protege.id}`);
  await d.page.locator('.fiche-entete__nom').waitFor({ timeout: 15000 });
  verifie(true, 'la direction (G6) ouvre la fiche du protégé');
  await d.ctx.close();
}

// ===== Professeur : périmètre =====
{
  const { ctx, page } = await ouvrir(prof);
  await page.goto(`${base}/web/eleves`);
  await page.getByText(/périmètre/).first().waitFor({ timeout: 15000 });
  verifie(true, 'le professeur est informé de son périmètre');
  await cap(page, '05-liste-professeur.png');
  await ctx.close();
}

// ===== Téléphone 375 px =====
for (const [nom, chemin, attente] of [['liste', '/web/eleves', '.eleves-tableau tbody tr'], ['fiche', null, '.fiche-entete__nom']]) {
  const { ctx, page } = await ouvrir(secretariat, 375, 800);
  let ch = chemin;
  if (!ch) { await page.goto(`${base}/web/eleves`); await page.waitForSelector('.eleves-nom a'); ch = new URL(await page.locator('.eleves-nom a').first().getAttribute('href'), base).pathname; }
  await page.goto(base + ch);
  await page.waitForSelector(attente, { timeout: 20000 });
  await page.waitForLoadState('networkidle');
  const debord = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  rapport.mobile.push({ page: nom, debordement_px: debord });
  verifie(debord <= 0, `375 px ${nom} : débordement ${debord}px`);
  await cap(page, `06-${nom}-375.png`);
  await axe(page, `${nom} 375 px`);
  await ctx.close();
}

verifie(rapport.erreurs.length === 0, `erreurs console / page : ${rapport.erreurs.length} ${rapport.erreurs.slice(0, 3).join(' | ')}`);
fs.writeFileSync(path.join(sortie, 'rapport.json'), JSON.stringify(rapport, null, 2));
await navigateur.close();
process.exit(rapport.echecs.length ? 1 : 0);
