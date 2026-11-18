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
async function settle(page, ms = 700) { await page.waitForLoadState('networkidle').catch(() => {}); await page.waitForTimeout(ms); }

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
  const alertes = await page.evaluate(() => [...document.querySelectorAll('[role=alert],[role=status],.ui-alerte')].map((a) => a.innerText.trim()).filter(Boolean).slice(0, 3));
  const f = await photo(page, cle, description);
  R.actions.push({ role: cle, domaine: page.domaine, description, statut, detail, retour: alertes, capture: f, erreurs: [...page.errs] });
  log(`  ▶ ${cle} · ${description} : ${statut}${detail ? ' — ' + detail : ''}${alertes.length ? ' [' + alertes.join(' / ').slice(0, 120) + ']' : ''}`);
}


// ---------- 1. visite de tous les écrans accessibles, rôle par rôle ----------
const ORDRE = ['direction', 'administratif', 'cpe', 'accueil', 'prof', 'prof_principal', 'infirmerie', 'compta', 'psy', 'personnel', 'parent', 'parent2', 'eleve_mineur', 'eleve_majeur', 'admin'];
const COMMUNS = new Set(['/web/securite', '/web/notifications', '/web/demandes', '/web/demandes/mes', '/web/mon-agenda', '/web/messages']);
const extras = {}; // chemins hors menu (fiches, sous-écrans), remplis plus bas
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

if (process.env.PHASE !== 'actions') {
  for (const cle of ORDRE) { if (seulement.length && !seulement.includes(cle)) continue; await visiteRole(cle).catch((e) => { R.erreurs.push(`${cle}: ${e.message}`); log(`ÉCHEC ${cle}: ${e.message}`); }); }
}
fs.writeFileSync(path.join(sortie, 'resultats.json'), JSON.stringify(R, null, 1));
await navigateur.close();
log(`terminé : ${R.ecrans.length} écrans, ${R.actions.length} actions, captures ${JSON.stringify(R.captures)}`);
