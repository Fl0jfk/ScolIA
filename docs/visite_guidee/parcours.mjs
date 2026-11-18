// Visite guidée ScolIA : parcours scripté, rôle par rôle, sur les données de labo (fictives).
// Prérequis : serveur ent-web-server (feature labo, ENT_ENV=lab) sur <base>, PostgreSQL local de test,
// Playwright global + Chromium déjà installés. Aucune écriture dans le code : on ne fait que jouer l'application.
// Usage : node parcours.mjs [base] [dossier-sortie] [rôles séparés par des virgules]
//   PGURL=postgres://… (facultatif) : une seule écriture SQL de labo (lien compte↔élève majeur), voir `lierCompteEleve`.
import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_PATH ?? '/opt/node22/lib/node_modules/playwright');

const base = process.argv[2] ?? 'http://127.0.0.1:8090';
const sortie = process.argv[3] ?? new URL('./2026-11', import.meta.url).pathname;
const seulement = (process.argv[4] ?? '').split(',').filter(Boolean);
fs.mkdirSync(sortie, { recursive: true });
const PGURL = process.env.PGURL ?? 'postgres://postgres@127.0.0.1:54329/ent_visite_test';

// ---------- journal ----------
const R = { ecrans: [], actions: [], erreurs: [], captures: {} };
const log = (m) => console.log(m);
const slug = (s) => s.toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, '').replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '').slice(0, 48);

// ---------- domaines (dossier par domaine) ----------
const DOMAINES = [
  ['00-accueil', (p) => p === '/web' || p.startsWith('/web/a-faire') || p.startsWith('/web/mes-files') || p.startsWith('/web/supervision-files') || p.startsWith('/web/mes-liens') || p.startsWith('/web/recherche') || p.startsWith('/web/blocages')],
  ['01-eleve', (p) => /^\/web\/(eleves|classes|mes-classes)/.test(p)],
  ['02-inscriptions', (p) => /^\/web\/(inscriptions|reinscription|preinscription|invitation)/.test(p)],
  ['03-assiduite', (p) => /^\/web\/(appel|assiduite|absences|journee|vie-scolaire|signalements|discipline)/.test(p)],
  ['04-absences-personnels', (p) => /^\/web\/(personnel|delegations|edt\/remplacements)/.test(p)],
  ['05-edt', (p) => /^\/web\/(emploi-du-temps|edt|changements-edt)/.test(p)],
  ['06-cahier', (p) => /^\/web\/cahier/.test(p)],
  ['07-evaluations', (p) => /^\/web\/evaluations/.test(p)],
  ['08-messagerie', (p) => /^\/web\/messages/.test(p)],
  ['09-agenda', (p) => /^\/web\/(agenda|mon-agenda|salles|rencontres|rendez-vous)/.test(p)],
  ['10-voyages', (p) => /^\/web\/voyages/.test(p)],
  ['11-demandes', (p) => /^\/web\/demandes/.test(p)],
  ['12-formulaires', (p) => /^\/web\/(formulaires|campagnes)/.test(p)],
  ['13-finances', (p) => /^\/web\/(comptabilite|mes-factures)/.test(p)],
  ['14-sante-accompagnement', (p) => /^\/web\/(infirmerie|sante-enfant|amenagements|accompagnement)/.test(p)],
  ['15-administration', (p) => /^\/web\/(referentiel|direction|securite|notifications|protections|roles)/.test(p)],
];
const domaineDe = (p) => (DOMAINES.find(([, f]) => f(p)) ?? ['99-autre'])[0];

// ---------- API ----------
async function api(chemin, { methode = 'GET', jeton, corps, tolerer } = {}) {
  const r = await fetch(base + chemin, {
    method: methode,
    headers: { 'content-type': 'application/json', ...(jeton ? { authorization: `Bearer ${jeton}` } : {}) },
    body: corps !== undefined ? JSON.stringify(corps) : undefined,
  });
  const t = await r.text();
  let j = null; try { j = JSON.parse(t); } catch { /* texte */ }
  if (!r.ok) { const e = new Error(`${methode} ${chemin} → ${r.status} ${t.slice(0, 160)}`); e.statut = r.status; if (tolerer) return null; throw e; }
  return j;
}
const tol = (chemin, o = {}) => api(chemin, { ...o, tolerer: true });

// ---------- jeu de données ----------
const labo = await api('/labo/bootstrap', { methode: 'POST', corps: {} });
const MDP = labo.password;
const etab = labo.etablissement_id;
const login = async (email) => ({ email, jeton: (await api('/auth/login', { methode: 'POST', corps: { email, password: MDP, etablissement_id: etab } })).token });
const perso = (role, i = 0) => labo.personnel.filter((p) => p.role === role)[i];
const eleveNom = (n) => labo.eleves.find((e) => e.prenom === n);

const admin = await login(labo.personas.find((p) => p.role === 'admin').email);
const comptes = {}; // clé de rôle de visite → { email, jeton, titre }
async function ajouterCompte(cle, email, titre) {
  comptes[cle] = { ...(await login(email)), titre };
}
async function creerCompte(role, local, nom) {
  const email = `${local}-${etab.slice(0, 8)}@ent.lab`;
  await api('/auth/register', { methode: 'POST', corps: { email, password: MDP, display_name: nom, etablissement_id: etab, role } });
  return email;
}
function lierCompteEleve(eleveId, email) {
  // Seule écriture hors API : en labo, aucun courriel ne part, donc l'invitation d'élève (503) ne peut pas aboutir.
  const sql = `INSERT INTO ent_eleve_compte (etablissement_id, eleve_id, user_id) SELECT '${etab}', '${eleveId}', id FROM ent_user WHERE email = '${email}'`;
  execFileSync('psql', [PGURL, '-v', 'ON_ERROR_STOP=1', '-q', '-c', sql]);
}

await ajouterCompte('direction', perso('direction').email, 'Direction');
await ajouterCompte('administratif', perso('administratif').email, 'Secrétariat');
await ajouterCompte('cpe', perso('cpe', 0).email, 'CPE / vie scolaire');
await ajouterCompte('compta', perso('compta').email, 'Comptabilité');
await ajouterCompte('infirmerie', perso('infirmerie').email, 'Infirmerie');
await ajouterCompte('psy', perso('psy').email, 'Psychologue');
await ajouterCompte('prof', perso('prof', 0).email, 'Professeur');
// Professeur principal : prof 2 désigné par la direction pour la classe 4B (PUT /referentiel/classes/{id}/professeur-principal).
const classes = await api('/referentiel/classes', { jeton: admin.jeton });
const c4b = classes.find((c) => c.nom === '4B' || c.code === '4B' || c.libelle === '4B') ?? classes[0];
await tol(`/referentiel/classes/${c4b.id}/professeur-principal`, { methode: 'PUT', jeton: admin.jeton, corps: { professeur_principal_id: perso('prof', 1).user_id, user_id: perso('prof', 1).user_id } });
// Séances datées du cahier de textes : l'administration les génère depuis l'emploi du temps (POST …/seances).
const annees = await tol('/referentiel/annees-statut', { jeton: admin.jeton });
for (const a of annees ?? []) { const r = await tol(`/referentiel/annees/${a.id}/seances`, { methode: 'POST', jeton: admin.jeton, corps: {} }); log(`séances générées pour ${a.libelle ?? a.id}: ${r ? JSON.stringify(r).slice(0, 100) : 'refusé'}`); }
await ajouterCompte('prof_principal', perso('prof', 1).email, 'Professeur principal');
for (const [cle, role, local, nom, titre] of [
  ['accueil', 'accueil', 'accueil1', 'Accueil Filch', 'Accueil'],
  ['personnel', 'personnel', 'aed1', 'AED Mondingus', 'Personnel (AED / agent)'],
]) { await ajouterCompte(cle, await creerCompte(role, local, nom), titre); }
await ajouterCompte('parent', labo.personas.find((p) => p.role === 'parent' && p.label.includes('Harry')).email, 'Parent (Harry)');
await ajouterCompte('parent2', labo.personas.find((p) => p.role === 'parent' && p.label.includes('Weasley')).email, 'Parent (Ron + Ginny)');
await ajouterCompte('eleve_mineur', labo.personas.find((p) => p.role === 'eleve').email, 'Élève mineur (Harry)');
const ambre = await api('/eleves', { methode: 'POST', jeton: admin.jeton, corps: { nom: 'Maturin', prenom: 'Ambre', date_naissance: '2007-03-04', classe: '4B' } });
lierCompteEleve(ambre.id, await creerCompte('eleve', 'ambre', 'Ambre Maturin'));
await ajouterCompte('eleve_majeur', `ambre-${etab.slice(0, 8)}@ent.lab`, 'Élève majeur (Ambre)');
await ajouterCompte('admin', labo.personas.find((p) => p.role === 'admin').email, 'Administrateur');
log(`établissement de labo ${etab} ; comptes : ${Object.keys(comptes).join(', ')}`);

// ---------- navigateur ----------
const navigateur = await chromium.launch();
async function ouvrir(cle, largeur = 1280) {
  const ctx = await navigateur.newContext({ viewport: { width: largeur, height: 900 }, locale: 'fr-FR', deviceScaleFactor: 1 });
  const page = await ctx.newPage();
  page.errs = [];
  page.on('pageerror', (e) => page.errs.push(`pageerror ${e.message.slice(0, 200)}`));
  page.on('console', (m) => { if (m.type() === 'error') page.errs.push(`console ${m.text().slice(0, 200)}`); });
  page.on('response', (r) => { if (r.status() >= 500) page.errs.push(`HTTP ${r.status()} ${r.url().replace(base, '')}`); });
  await page.goto(`${base}/web/connexion?etab=${etab}`);
  await page.waitForSelector('#email');
  await page.fill('#email', comptes[cle].email);
  await page.fill('#mdp', MDP);
  if (await page.locator('#etab').count()) await page.fill('#etab', etab);
  await page.locator('form button[type=submit]').last().click();
  await page.waitForURL((u) => !u.pathname.includes('connexion'), { timeout: 60000 });
  await settle(page);
  return { ctx, page };
}
async function settle(page, ms = 700) {
  await page.waitForLoadState('networkidle', { timeout: 8000 }).catch(() => {});
  await page.waitForTimeout(ms);
  // Écrans lents : on attend la fin d'un « Chargement… » (25 s au plus) avant de photographier.
  for (let i = 0; i < 25; i++) {
    const charge = await page.evaluate(() => /Chargement (de|des|du|d')/.test((document.querySelector('main') ?? document.body).innerText)).catch(() => false);
    if (!charge) break;
    await page.waitForTimeout(1000);
  }
}

const compteurs = {};
async function photo(page, cle, nom, { mobile = false } = {}) {
  const dossier = path.join(sortie, page.domaine ?? '99-autre');
  fs.mkdirSync(dossier, { recursive: true });
  compteurs[cle] = (compteurs[cle] ?? 0) + 1;
  const f = `${cle.replace(/_/g, '-')}-${String(compteurs[cle]).padStart(2, '0')}-${slug(nom)}${mobile ? '-375' : ''}.png`;
  const vp = page.viewportSize();
  const hauteur = await page.evaluate(() => Math.max(document.documentElement.scrollHeight, document.querySelector('main')?.scrollHeight ?? 0, ...[...document.querySelectorAll('main *')].slice(0, 4000).map((e) => e.scrollHeight && e.getBoundingClientRect().bottom + window.scrollY)));
  const cible = Math.min(Math.max(vp.height, Math.ceil(hauteur) + 40), 3000);
  if (cible > vp.height) { await page.setViewportSize({ width: vp.width, height: cible }); await page.waitForTimeout(250); }
  await page.screenshot({ path: path.join(dossier, f) }).catch(() => {});
  if (cible > vp.height) await page.setViewportSize(vp);
  const d = page.domaine ?? '99-autre';
  R.captures[d] = (R.captures[d] ?? 0) + 1;
  return `${d}/${f}`;
}

// Lecture de l'écran : titre, texte, états remarquables.
async function lire(page) {
  return page.evaluate(() => {
    const main = document.querySelector('main') ?? document.body;
    const txt = main.innerText.replace(/\s+/g, ' ').trim();
    return {
      titre: (main.querySelector('h1')?.innerText ?? document.title).trim(),
      texte: txt.slice(0, 600),
      longueur: txt.length,
      boutons: [...main.querySelectorAll('button')].map((b) => b.innerText.trim()).filter(Boolean).slice(0, 25),
      champs: main.querySelectorAll('input,select,textarea').length,
      alertes: [...main.querySelectorAll('[role=alert],.ui-alerte')].map((a) => a.innerText.trim().slice(0, 160)),
      lignes: main.querySelectorAll('tr,li').length,
    };
  });
}
const PAS_DISPO = /pas encore disponible|bient[oô]t disponible|non disponible|n'existe pas encore|à venir|pas encore (branch|impl)/i;
const ERREUR = /erreur|impossible|refus|introuvable|interdit|403|404|500|indisponible/i;

async function visiter(page, cle, chemin, nom, { mobile = true, motif } = {}) {
  page.errs.length = 0;
  page.domaine = domaineDe(chemin.split('?')[0]);
  const t0 = Date.now();
  await page.goto(base + chemin).catch((e) => page.errs.push(`goto ${e.message.slice(0, 100)}`));
  await settle(page, 900);
  const url = new URL(page.url());
  const lu = await lire(page);
  const redirige = url.pathname !== chemin.split('?')[0];
  const f = await photo(page, cle, nom);
  const ecran = { role: cle, chemin, nom, domaine: page.domaine, capture: f, redirige: redirige ? url.pathname : null, ms: Date.now() - t0, ...lu, erreurs: [...page.errs] };
  ecran.signaux = [
    ...(PAS_DISPO.test(lu.texte) ? ['message « pas encore disponible »'] : []),
    ...(lu.alertes.length ? [`alerte : ${lu.alertes[0]}`] : []),
    ...(/Chargement (de|des|du|d')/.test(lu.texte) ? ['reste en « Chargement… » après 25 s'] : []),
    ...(lu.longueur < 120 ? ['écran presque vide'] : []),
    ...(redirige ? [`redirigé vers ${url.pathname}`] : []),
    ...page.errs.slice(0, 2),
  ];
  R.ecrans.push(ecran);
  log(`  ${cle} ${chemin} → ${lu.titre} ${ecran.signaux.length ? '⚠ ' + ecran.signaux.join(' | ') : ''}`);
  if (mobile) {
    await page.setViewportSize({ width: 375, height: 800 });
    await page.waitForTimeout(400);
    await photo(page, cle, nom, { mobile: true });
    await page.setViewportSize({ width: 1280, height: 900 });
  }
  return ecran;
}

// Action métier jouée : capture avant / après, résultat honnête (ok | échec + raison).
async function agir(page, cle, description, fn, { domaine } = {}) {
  page.errs.length = 0;
  if (domaine) page.domaine = domaine;
  let statut = 'ok'; let detail = '';
  try { await fn(); } catch (e) { statut = 'échec'; detail = String(e.message).split('\n')[0].slice(0, 220); }
  await settle(page, 800);
  const alertes = await page.evaluate(() => [...new Set([...document.querySelectorAll('[role=alert],[role=status],.ui-alerte')].map((a) => a.innerText.trim().replace(/\s+/g, ' ').replace(/×$/, '').trim()).filter(Boolean))].slice(0, 3));
  if (statut === 'ok' && alertes.some((a) => /refus|impossible|invalide|interdit|erreur|corrigez|introuvable|indisponible/i.test(a))) { statut = 'refusé par le serveur'; detail = alertes.find((a) => /refus|impossible|invalide|interdit|erreur|corrigez|introuvable|indisponible/i.test(a)).slice(0, 220); }
  const f = await photo(page, cle, description);
  R.actions.push({ role: cle, domaine: page.domaine, description, statut, detail, retour: alertes, capture: f, erreurs: [...page.errs] });
  log(`  ▶ ${cle} · ${description} : ${statut}${detail ? ' — ' + detail : ''}${alertes.length ? ' [' + alertes.join(' / ').slice(0, 120) + ']' : ''}`);
}


// Aide à l'écriture des scénarios : liste ce qu'un écran propose (boutons, champs, onglets).
async function decrire(page) {
  return page.evaluate(() => {
    const m = document.querySelector('main') ?? document.body;
    const nom = (e) => (e.getAttribute('aria-label') || e.labels?.[0]?.innerText || e.innerText || e.placeholder || e.value || '').replace(/\s+/g, ' ').trim().slice(0, 60);
    return [...m.querySelectorAll('button,a[href],input,select,textarea,[role=tab],[role=radio]')].map((e) => `${e.tagName.toLowerCase()}${e.type ? ':' + e.type : ''}${e.id ? '#' + e.id : ''} «${nom(e)}»${e.disabled ? ' (désactivé)' : ''}`);
  });
}
if (process.env.INSPECT) {
  for (const spec of process.env.INSPECT.split(';')) {
    const [cle, chemin] = spec.split('@');
    const { ctx, page } = await ouvrir(cle);
    await page.goto(base + chemin); await settle(page, 1200);
    console.log(`\n## ${cle} ${chemin}\n` + (await decrire(page)).join('\n'));
    await ctx.close();
  }
  await navigateur.close(); process.exit(0);
}

// ---------- 1. visite de tous les écrans accessibles, rôle par rôle ----------
const ORDRE = ['direction', 'administratif', 'cpe', 'accueil', 'prof', 'prof_principal', 'infirmerie', 'compta', 'psy', 'personnel', 'parent', 'parent2', 'eleve_mineur', 'eleve_majeur', 'admin'];
const COMMUNS = new Set(['/web/securite', '/web/notifications', '/web/demandes', '/web/demandes/mes', '/web/mon-agenda', '/web/messages']);
const harry = eleveNom('Harry');
const FICHE = [[`/web/classes/${c4b.id}`, 'fiche classe 4B'], [`/web/eleves/${harry.id}`, 'fiche élève Harry'], [`/web/eleves/${harry.id}/dossier`, 'dossier élève Harry'],
  [`/web/assiduite/eleves/${harry.id}`, 'dossier assiduité Harry'], [`/web/evaluations/classes/${c4b.id}`, 'évaluations de la classe 4B'], [`/web/evaluations/eleves/${harry.id}`, 'évaluations de Harry'],
  [`/web/accompagnement/eleves/${harry.id}`, 'accompagnement de Harry'], [`/web/infirmerie/eleves/${harry.id}`, 'infirmerie : dossier de Harry']];
const extras = { // chemins hors menu (fiches, sous-écrans)
  direction: [...FICHE, ['/web/referentiel/annees', 'référentiel : années'], ['/web/referentiel/structure', 'référentiel : structure'], ['/web/referentiel/classes', 'référentiel : classes'], ['/web/referentiel/calendrier', 'référentiel : calendrier'], ['/web/referentiel/grille', 'référentiel : grille'], ['/web/direction/securite', 'politique de sécurité'], ['/web/direction/protections', 'protections'], ['/web/direction/roles/groupes', 'rôles : responsables de groupes'], ['/web/salles', 'salles'], ['/web/cahier/charge', 'cahier : charge'], ['/web/voyages/nouveau', 'voyages : nouvelle sortie'], ['/web/voyages/serie', 'voyages : nouvelle série'], ['/web/reinscription', 'réinscription'], ['/web/blocages-recherche', 'recherches bloquées'], ['/web/personnel/mes-heures', 'mes heures']],
  cpe: FICHE.slice(0, 5).concat([['/web/salles', 'salles']]),
  prof: [['/web/classes', 'classes'], [`/web/classes/${c4b.id}`, 'fiche classe 4B'], ['/web/cahier/cours', 'cahier du cours'], ['/web/cahier/travail', 'donner du travail'], ['/web/cahier/charge', 'charge de travail'], ['/web/salles', 'salles'], ['/web/personnel/mes-heures', 'mes heures'], ['/web/personnel/absences/declarer', 'déclarer une absence'], ['/web/voyages/nouveau', 'nouvelle sortie'], ['/web/mes-liens', 'mes liens']],
  prof_principal: [[`/web/classes/${c4b.id}`, 'fiche classe 4B (professeur principal)'], [`/web/evaluations/classes/${c4b.id}`, 'évaluations de la classe 4B']],
  administratif: [...FICHE.slice(0, 3), ['/web/reinscription', 'réinscription'], ['/web/referentiel/classes', 'référentiel : classes'], ['/web/salles', 'salles']],
  infirmerie: [[`/web/infirmerie/eleves/${harry.id}`, 'infirmerie : dossier de Harry'], [`/web/infirmerie/eleves/${harry.id}/urgence`, 'fiche d’urgence de Harry']],
  parent: [['/web/reinscription', 'réinscription'], ['/web/rendez-vous', 'rendez-vous'], ['/web/mes-liens', 'mes liens'], ['/web/formulaires', 'formulaires'], ['/web/cahier/agenda', 'mon cahier']],
  compta: [['/web/mes-factures', 'factures (vue famille)']],
  eleve_majeur: [['/web/reinscription', 'réinscription']],
};
const vusCommuns = new Set();
async function visiteRole(cle) {
  const { ctx, page } = await ouvrir(cle);
  log(`\n=== ${cle} (${comptes[cle].titre}) ===`);
  await visiter(page, cle, '/web', 'accueil');
  const liens = await page.locator('nav[aria-label="Navigation principale"] a[href^="/web"]').evaluateAll((a) => a.map((x) => [x.getAttribute('href'), x.textContent.trim().replace(/\d+\+?$/, '')]));
  R.menus ??= {}; R.menus[cle] = liens;
  for (const [href, nom] of liens) {
    if (href === '/web') continue;
    const commun = COMMUNS.has(href);
    if (commun && vusCommuns.has(href + '|' + (cle.startsWith('eleve') || cle.startsWith('parent') ? 'f' : 'p'))) { continue; }
    vusCommuns.add(href + '|' + (cle.startsWith('eleve') || cle.startsWith('parent') ? 'f' : 'p'));
    await visiter(page, cle, href, nom, { mobile: true });
  }
  for (const [href, nom] of extras[cle] ?? []) await visiter(page, cle, href, nom, { mobile: false });
  await ctx.close();
}

// ---------- 2. actions métier jouées, domaine par domaine ----------
// Chaque scénario : { id, cle (rôle), domaine, fn(page, ctx) } ; il enchaîne des `agir` (capture + résultat honnête).
const SCENARIOS = [];
const scenario = (id, cle, fn) => SCENARIOS.push({ id, cle, fn });
const cliquer = (page, nom, opts = {}) => page.getByRole(opts.role ?? 'button', { name: nom, exact: opts.exact ?? false }).first().click({ timeout: 8000 });
const remplir = (page, etiquette, valeur) => page.getByLabel(etiquette, { exact: false }).first().fill(valeur, { timeout: 8000 });
const aller = async (page, chemin) => { page.domaine = domaineDe(chemin); await page.goto(base + chemin); await settle(page, 900); };
const jourPasse = (n) => { const d = new Date(Date.now() - n * 864e5); return d.toISOString().slice(0, 10); };
const ctxS = { jourAppel: null };

const voir = async (page, t = '') => { if (process.env.DEBUG) console.log(`   [voir ${t}]\n    ` + (await decrire(page)).slice(2).join('\n    ')); };

// --- Vie scolaire : l'appel (professeur)
scenario('appel', 'prof', async (page) => {
  const j = jourPasse(7); // même jour de semaine, séance non encore appelée
  await aller(page, '/web/appel');
  await page.fill('#jour', j); await settle(page);
  await agir(page, 'prof', "appel : choisir la séance (liste du jour)", async () => { await page.locator('a', { hasText: 'Potions' }).first().click(); await settle(page); });
  await agir(page, 'prof', "appel : Harry absent, Ginny en retard", async () => {
    await page.getByRole('radio', { name: 'Absent' }).nth(1).check();
    await page.getByRole('radio', { name: 'Retard' }).nth(0).check();
    await page.locator('input[id^=min-]').first().fill('10');
  });
  await agir(page, 'prof', "appel : valider « Appel fait »", async () => {
    await cliquer(page, 'Appel fait', { exact: true });
    await settle(page, 600);
    const c = page.getByRole('button', { name: /Confirmer : appel fait/ });
    if (await c.count()) await c.first().click();
  });
  await agir(page, 'prof', "appel : corriger un statut après validation", async () => {
    await voir(page, 'après validation');
    const b = page.getByRole('button', { name: /Corriger/ });
    await b.first().click({ timeout: 4000 });
    await page.selectOption('#corr-etat', { index: 1 }).catch(() => {});
    await page.selectOption('#corr-motif', { index: 1 }).catch(() => {});
    await cliquer(page, 'Enregistrer la correction');
  });
});

// --- Vie scolaire : la famille déclare, la vie scolaire décide
scenario('abs-parent', 'parent', async (page) => {
  await aller(page, '/web/absences');
  await agir(page, 'parent', "absences : donner le motif d'une absence relevée", async () => {
    await page.getByRole('button', { name: 'Donner le motif' }).first().click();
    await page.selectOption('#abs-motif', { index: 1 });
    await page.fill('#abs-precision', 'Rendez-vous médical (jeu de labo).');
    await cliquer(page, 'Envoyer le motif');
  });
  await agir(page, 'parent', "absences : déclarer une absence à l'avance", async () => {
    await cliquer(page, 'Déclarer une absence');
    const d = new Date(Date.now() + 2 * 864e5).toISOString().slice(0, 10);
    await page.fill('#decl-debut-jour', d);
    await page.selectOption('#decl-motif', { index: 1 });
    await page.fill('#decl-commentaire', 'Cérémonie familiale (jeu de labo).');
    await page.getByRole('button', { name: 'Enregistrer', exact: true }).click();
  });
});
scenario('abs-cpe', 'cpe', async (page) => {
  await aller(page, '/web/vie-scolaire');
  await agir(page, 'cpe', "vie scolaire : examiner l'absence avec justificatif", async () => { await cliquer(page, 'Examiner'); await voir(page, 'examiner'); });
});
// --- Cahier de textes : le professeur donne du travail, l'élève le voit
scenario('cahier-prof', 'prof', async (page) => {
  await aller(page, '/web/cahier/travail');
  await agir(page, 'prof', "cahier : donner du travail à la classe (publié)", async () => {
    await page.locator('input[id^=cdt-d-cours-]').first().check();
    await page.selectOption('#cdt-d-nature', { index: 1 });
    await page.fill('#cdt-d-titre', 'Exercices de potions, page 42');
    await page.fill('#cdt-d-consigne', 'Faire les exercices 1 à 4 et relire la leçon sur les décoctions.');
    await page.fill('#cdt-d-duree', '30');
    await page.locator('#cdt-d-echeance-date').check(); await page.fill('#cdt-d-jour', new Date(Date.now() + 6 * 864e5).toISOString().slice(0, 10));
    await page.locator('#cdt-d-publication-maintenant').check();
    await page.getByRole('button', { name: 'Enregistrer le travail' }).click();
  });
  await aller(page, '/web/cahier/cours');
  await agir(page, 'prof', "cahier : ouvrir le cahier du cours", async () => { await page.locator('a', { hasText: 'Potions' }).first().click(); await settle(page); await voir(page, 'cahier-cours'); });
  await aller(page, '/web/cahier/charge');
  await agir(page, 'prof', "cahier : charge de travail des classes", async () => { await voir(page, 'charge'); });
});
scenario('cahier-eleve', 'eleve_mineur', async (page) => {
  await aller(page, '/web/agenda');
  await agir(page, 'eleve_mineur', "élève : travail à faire (agenda)", async () => { await voir(page, 'agenda-eleve'); });
  await aller(page, '/web/cahier/agenda');
  await agir(page, 'eleve_mineur', "élève : mon cahier (séances et contenus)", async () => { await voir(page, 'mon-cahier'); });
});

// --- Messagerie : la famille écrit, le professeur répond
scenario('msg-parent', 'parent', async (page) => {
  await aller(page, '/web/messages/nouveau');
  await agir(page, 'parent', "messagerie : écrire à l'établissement (relire puis envoyer)", async () => {
    await page.fill('#msg-sujet', 'Question sur les devoirs de potions');
    await page.fill('#msg-corps', 'Bonjour, Harry a-t-il bien noté les exercices pour la semaine prochaine ? Merci.');
    await page.getByRole('button', { name: 'Relire et envoyer' }).click();
    await settle(page, 500);
    await cliquer(page, 'Envoyer le message');
  });
});
scenario('msg-prof', 'prof', async (page) => {
  await aller(page, '/web/messages');
  await agir(page, 'prof', "messagerie : boîte de réception du professeur", async () => { await voir(page, 'inbox-prof'); });
  await aller(page, '/web/messages/nouveau');
  await agir(page, 'prof', "messagerie : nouveau message (écran de rédaction)", async () => { await voir(page, 'nouveau-prof'); });
});

// --- Demandes : la famille demande, le secrétariat traite
scenario('dem-parent', 'parent', async (page) => {
  await aller(page, '/web/demandes');
  await agir(page, 'parent', "demandes : choisir « Certificat de scolarité »", async () => { await page.locator('a', { hasText: 'Certificat de scolarité' }).first().click(); await settle(page); await voir(page, 'demande-formulaire'); });
});
scenario('dem-admin', 'administratif', async (page) => {
  await aller(page, '/web/demandes/file');
  await agir(page, 'administratif', "demandes : ma file de traitement", async () => { await voir(page, 'ma-file'); });
});

// --- Absences des personnels : le professeur déclare, la direction instruit
scenario('absp-prof', 'prof', async (page) => {
  await aller(page, '/web/personnel/absences/declarer');
  await agir(page, 'prof', "absences du personnel : déclarer une absence demain", async () => {
    await page.locator('#decl-quand-demain').check();
    await page.locator('#decl-partie-journee').check();
    await page.selectOption('#decl-nature', { index: 1 });
    await page.getByRole('button', { name: "Déclarer l'absence" }).click();
  });
  await aller(page, '/web/personnel/absences');
  await agir(page, 'prof', "absences du personnel : voir mes absences", async () => { await voir(page, 'mes-absences'); });
});
scenario('absp-direction', 'direction', async (page) => {
  await aller(page, '/web/personnel/instruction-absences');
  await agir(page, 'direction', "absences du personnel : file d'instruction", async () => { await voir(page, 'instruction'); });
});
scenario('abs-cpe2', 'cpe', async (page) => {
  await aller(page, '/web/vie-scolaire');
  await agir(page, 'cpe', "vie scolaire : examiner l'absence de Harry", async () => { await page.getByRole('button', { name: /Examiner l'absence de Harry/ }).first().click(); await settle(page); });
  await agir(page, 'cpe', "vie scolaire : accepter le justificatif", async () => { await cliquer(page, 'Accepter le justificatif'); });
  await aller(page, '/web/discipline');
  await agir(page, 'cpe', "discipline : rechercher l'élève", async () => { await page.fill('#disc-eleve-q', 'Potter'); await page.keyboard.press('Enter'); await settle(page, 1200); await page.keyboard.press('Escape'); await voir(page, 'discipline-resultat'); });
  await agir(page, 'cpe', "discipline : ouvrir le dossier de l'élève", async () => { await page.getByRole('button', { name: /Potter/ }).first().click(); await settle(page); await voir(page, 'discipline-eleve'); });
});
scenario('infirmerie', 'infirmerie', async (page) => {
  await aller(page, '/web/infirmerie');
  await agir(page, 'infirmerie', "infirmerie : chercher un élève pour un passage", async () => { await page.fill('#pas-eleve-q', 'Potter'); await page.getByRole('button', { name: 'Rechercher' }).first().click(); await settle(page); await voir(page, 'passage'); });
});
scenario('eleve-semaine', 'eleve_mineur', async (page) => {
  await aller(page, '/web/cahier/agenda');
  await agir(page, 'eleve_mineur', "élève : semaine suivante (travail donné par le professeur)", async () => { await page.getByRole('link', { name: 'Semaine suivante' }).first().click(); await settle(page, 1200); });
});
scenario('msg-admin', 'administratif', async (page) => {
  await aller(page, '/web/messages');
  await agir(page, 'administratif', "messagerie : ouvrir la conversation d'une famille", async () => { await page.getByText(/devoirs de potions/).first().click(); await settle(page, 1200); await voir(page, 'conversation'); });
});
scenario('palette', 'direction', async (page) => {
  await aller(page, '/web');
  await agir(page, 'direction', "palette « Rechercher ou agir » (Ctrl K)", async () => { await page.keyboard.press('Control+k'); await settle(page, 600); await page.keyboard.type('absence'); await settle(page, 900); });
  await page.keyboard.press('Escape');
});
scenario('dem-parent2', 'parent', async (page) => {
  await aller(page, '/web/demandes');
  await page.locator('a', { hasText: 'Certificat de scolarité' }).first().click(); await settle(page, 1800);
  await agir(page, 'parent', "demandes : formulaire « Certificat de scolarité »", async () => { await voir(page, 'demande-form'); });
});
scenario('eval-prof', 'prof', async (page) => {
  await aller(page, '/web/evaluations');
  await agir(page, 'prof', "évaluations : choisir la période et ouvrir la grille", async () => { await page.selectOption('#eva-periode', { index: 2 }).catch(() => {}); await settle(page); await page.getByRole('link', { name: /Ouvrir la grille/ }).first().click(); await settle(page, 1500); await voir(page, 'grille'); });
});
scenario('voyage', 'direction', async (page) => {
  await aller(page, '/web/voyages/nouveau');
  await agir(page, 'direction', "voyages : créer un brouillon de sortie", async () => {
    await page.fill('#voy-titre', 'Sortie au musée de la magie');
    await page.selectOption('#voy-nature', { index: 1 }); await page.selectOption('#voy-forme', { index: 1 }); await page.selectOption('#voy-destination', { index: 1 });
    await page.fill('#voy-lieu', 'Musée de la magie, Paris');
    const d = (n) => new Date(Date.now() + n * 864e5).toISOString().slice(0, 10);
    await page.fill('#voy-date-debut', d(30)); await page.fill('#voy-heure-debut', '08:30');
    await page.fill('#voy-date-fin', d(30)); await page.fill('#voy-heure-fin', '17:00');
    await page.fill('#voy-effectif', '28'); await page.fill('#voy-adultes', '3');
    await page.fill('#voy-rdv', 'Cour du collège'); await page.fill('#voy-objectifs', 'Découvrir le patrimoine culturel.');
    await page.locator('input[id^=voy-classe-]').nth(1).check();
    await page.getByRole('button', { name: 'Créer le brouillon' }).click();
  });
  await voir(page, 'voyage-fiche');
});
scenario('formulaire', 'direction', async (page) => {
  await aller(page, '/web/formulaires');
  await agir(page, 'direction', "formulaires : créer un brouillon de consentement", async () => {
    await page.selectOption('#form-type', { index: 1 });
    await page.fill('#form-objet', 'sortie_musee'); await page.fill('#form-titre', 'Autorisation de sortie au musée');
    await page.fill('#form-texte', "J'autorise mon enfant à participer à la sortie au musée.");
    await page.selectOption('#form-audience', { index: 0 }).catch(() => {});
    await page.getByRole('button', { name: 'Créer le brouillon' }).click();
  });
});
scenario('campagne', 'direction', async (page) => {
  await aller(page, '/web/campagnes');
  await agir(page, 'direction', "campagnes : préparer une information aux familles", async () => {
    await page.fill('#camp-titre', 'Réunion de rentrée'); await page.selectOption('#camp-classe', { index: 1 }); await page.fill('#camp-corps', 'Rappel : réunion parents-professeurs le jeudi à 18 h.');
    await page.locator('#camp-email').check();
    await page.getByRole('button', { name: 'Voir les destinataires' }).click();
  });
});
scenario('rencontres', 'direction', async (page) => {
  await aller(page, '/web/rencontres');
  await agir(page, 'direction', "rencontres : créer une rencontre parents-professeurs", async () => { await page.fill('#renc-titre', 'Rencontres parents-professeurs 4B'); await page.selectOption('#renc-classe', { index: 1 }); await page.getByRole('button', { name: 'Créer la rencontre' }).click(); });
});
scenario('inscr', 'direction', async (page) => {
  await aller(page, '/web/inscriptions');
  await agir(page, 'direction', "inscriptions : créer une campagne d'admission", async () => {
    await page.getByText('Nouvelle campagne', { exact: true }).first().click().catch(() => {}); await page.waitForTimeout(500); await page.fill('#ins-libelle', 'Admissions 2027-2028'); await page.fill('#ins-niveaux', '6e, 5e');
    
    await page.locator('#ins-libelle').press('Enter');
  });
});
scenario('role', 'direction', async (page) => {
  await aller(page, '/web/direction/roles/nouveau');
  await agir(page, 'direction', "rôles : créer un rôle « Professeur de théâtre »", async () => {
    await page.fill('#role-nom', 'Professeur de théâtre'); await page.fill('#role-description', 'Atelier théâtre : voit les absences de ses élèves.');
    await cliquer(page, 'Créer le rôle');
  });
});
scenario('eleves', 'direction', async (page) => {
  await aller(page, '/web/eleves');
  await agir(page, 'direction', "élèves : rechercher puis ouvrir une fiche", async () => { await page.getByRole('searchbox').first().fill('Granger'); await settle(page); await page.locator('a', { hasText: 'Granger' }).first().click(); await settle(page, 1500); });
  await agir(page, 'direction', "élèves : dossier administratif complet", async () => { const u = page.url(); await page.goto(u.replace(/\/?$/, '') + '/dossier'); await settle(page, 1500); });
});
scenario('compta', 'compta', async (page) => {
  await aller(page, '/web/comptabilite');
  for (const onglet of ['Prélèvements', 'Relances', 'Tarifs', 'Export comptable'])
    await agir(page, 'compta', `comptabilité : onglet ${onglet}`, async () => { await page.getByRole('radio', { name: onglet }).check(); await settle(page); });
});
scenario('agenda', 'prof', async (page) => {
  await aller(page, '/web/mon-agenda');
  await agir(page, 'prof', "agenda : créer un événement (brouillon)", async () => {
    await cliquer(page, 'Nouvel événement'); await page.fill('#agd-titre', 'Réunion équipe 4B');
    await page.fill('#agd-jour', new Date(Date.now() + 3 * 864e5).toISOString().slice(0, 10)); await page.fill('#agd-h-debut', '17:00'); await page.fill('#agd-h-fin', '18:00'); await page.fill('#agd-lieu', 'Salle des professeurs');
    await page.getByRole('button', { name: 'Créer le brouillon' }).click();
  });
});
//@SCENARIOS
async function jouerScenarios() {
  const parCle = {};
  for (const sc of SCENARIOS) { if (process.env.SCENARIO && !process.env.SCENARIO.split(',').includes(sc.id)) continue; (parCle[sc.cle] ??= []).push(sc); }
  for (const [cle, liste] of Object.entries(parCle)) {
    const { ctx, page } = await ouvrir(cle);
    log(`\n=== actions ${cle} ===`);
    for (const sc of liste) { try { await sc.fn(page, ctxS); } catch (e) { R.erreurs.push(`scénario ${sc.id}: ${e.message}`); log(`ÉCHEC scénario ${sc.id}: ${e.message.slice(0, 200)}`); } }
    await ctx.close();
  }
}

if (process.env.PHASE !== 'actions') {
  for (const cle of ORDRE) { if (seulement.length && !seulement.includes(cle)) continue; await visiteRole(cle).catch((e) => { R.erreurs.push(`${cle}: ${e.message}`); log(`ÉCHEC ${cle}: ${e.message}`); }); }
}
if (process.env.PHASE !== 'visite') await jouerScenarios();
fs.writeFileSync(path.join(sortie, 'resultats.json'), JSON.stringify(R, null, 1));
await navigateur.close();
log(`terminé : ${R.ecrans.length} écrans, ${R.actions.length} actions, captures ${JSON.stringify(R.captures)}`);
