// Démo P1 jouée dans un navigateur (Playwright préinstallé). Données de labo fictives, base locale.
//   ./reset_labo.sh && NODE_PATH=$(npm root -g) node demo_p1.js
// Captures à deviceScaleFactor 2 dans le dossier parent. Aides de labo (pas de messagerie en labo) :
// code de préinscription retrouvé par code_labo.py ; jeton d'invitation et mot de passe de la mère
// posés en base locale. Aucune de ces aides n'existe en production.
const { ouvrir, connecter, persona, l, BASE } = require('./lib');
const { execSync } = require('child_process');
const fs = require('fs');
const OUT = '..';
const PSQL = `PGPASSWORD=ci psql -h 127.0.0.1 -U ent_ci -d ent_ci_test_a -Atc`;
const sql = q => execSync(`${PSQL} "${q}"`).toString().trim();
const shot = (p, nom) => p.screenshot({ path: `${OUT}/${nom}.png`, fullPage: true });
const log = (...a) => console.log(...a);
const api = async (token, method, path, body) => {
  const r = await fetch(BASE + path, { method, headers: { 'content-type': 'application/json', authorization: 'Bearer ' + token }, body: body ? JSON.stringify(body) : undefined });
  return r.status < 300 && r.status !== 204 ? r.json() : r.status;
};
const jeton = async email => (await (await fetch(BASE + '/auth/login', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ email, password: l.password, etablissement_id: l.etablissement_id }) })).json()).token;
const MERE = 'ines.martin@exemple.lab';

async function dossierOuvert(p) {
  await p.goto(`${BASE}/web/inscriptions`, { waitUntil: 'networkidle' }); await p.waitForTimeout(1500);
  await p.getByRole('button', { name: 'Dossiers' }).first().click(); await p.waitForTimeout(1200);
  await p.getByRole('button', { name: 'Instruire' }).first().click(); await p.waitForTimeout(1500);
}

(async () => {
  let b, p;
  // ---- 1. Secrétariat : campagne -------------------------------------------------------------
  ({ b, p } = await ouvrir()); await connecter(p, persona('administratif'));
  await p.goto(`${BASE}/web/inscriptions`, { waitUntil: 'networkidle' }); await p.waitForTimeout(1500);
  await p.locator('summary', { hasText: 'Nouvelle campagne' }).click();
  await p.getByLabel('Libellé').fill('Rentrée 2027 — admissions');
  await p.getByLabel(/Année cible/).fill('2027-2028');
  await p.getByLabel('Niveaux ouverts').fill('6EME:30, 5EME:20, 4EME:20');
  await p.getByLabel('Pièces demandées').fill("livret ; Livret scolaire\ncni ; Pièce d'identité du responsable\nvaccins ; Carnet de vaccination ; facultative");
  await shot(p, '01-secretariat-nouvelle-campagne');
  await p.getByRole('button', { name: 'Créer la campagne' }).click(); await p.waitForTimeout(1500);
  await p.getByRole('button', { name: 'Ouvrir', exact: true }).click(); await p.waitForTimeout(800);
  await shot(p, '02a-secretariat-confirmation-ouverture'); // n° 10 : confirmation avant d'ouvrir
  await p.getByRole('button', { name: 'Ouvrir la campagne' }).click(); await p.waitForTimeout(1500);
  await shot(p, '02-secretariat-campagne-ouverte'); await b.close();

  // ---- 2. Famille : préinscription -----------------------------------------------------------
  ({ b, p } = await ouvrir()); 
  await p.goto(`${BASE}/web/preinscription?etab=${l.etablissement_id}`, { waitUntil: 'networkidle' }); await p.waitForTimeout(2000);
  await p.getByLabel('Votre adresse e-mail').fill(MERE);
  await p.getByRole('button', { name: 'Continuer' }).click(); await p.waitForTimeout(1500);
  await p.getByLabel(/Code de vérification/).fill(execSync(`python3 code_labo.py ${MERE}`).toString().trim());
  await p.getByRole('button', { name: 'Vérifier' }).click(); await p.waitForTimeout(2000);
  await shot(p, '03-famille-espace');
  await p.getByRole('button', { name: 'Déposer une demande' }).click(); await p.waitForTimeout(800);
  await p.locator('#pi-eleve-nom').fill('Martin'); await p.locator('#pi-eleve-prenom').fill('Rose');
  await p.locator('#pi-eleve-naissance').fill('2013-05-12'); await p.locator('#pi-niveau').selectOption('4EME');
  await p.locator('#pi-resp-nom').fill('Martin'); await p.locator('#pi-resp-prenom').fill('Inès');
  await p.locator('#pi-resp-tel').fill('0611223344'); await p.locator('#pi-resp-adresse').fill('12 rue des Lilas, 69000 Lyon');
  await p.locator('#pi-resp-lien').selectOption('mere'); await p.locator('#pi-resp-protegee').check();
  await p.locator('#pi-second').check(); await p.waitForTimeout(300);
  await p.locator('#pi-second-nom').fill('Martin'); await p.locator('#pi-second-prenom').fill('Marc');
  await p.locator('#pi-second-lien').selectOption('pere'); await p.locator('#pi-second-foyer').uncheck();
  await p.locator('#pi-second-email').fill('marc.martin@exemple.lab'); await p.locator('#pi-second-nps').check();
  await shot(p, '04-famille-formulaire-reserves');
  await p.getByRole('button', { name: 'Envoyer ma demande' }).click(); await p.waitForTimeout(2500);
  for (const f of ['carte-identite-ines.pdf', 'livret-rose.pdf']) {
    await p.getByLabel(/Nom du fichier pour/).fill(f);
    await p.getByRole('button', { name: 'Déclarer la pièce' }).click(); await p.waitForTimeout(1500);
  }
  await shot(p, '05-famille-pieces-deposees'); await b.close();
  // Même formulaire à 375 px
  ({ b, p } = await ouvrir({ viewport: { width: 375, height: 812 } }));
  await p.goto(`${BASE}/web/preinscription?etab=${l.etablissement_id}`, { waitUntil: 'networkidle' }); await p.waitForTimeout(1500);
  await shot(p, '05b-famille-375px-accueil'); await b.close();

  // ---- 3. Instruction, décision, validation ----------------------------------------------------
  ({ b, p } = await ouvrir()); await connecter(p, persona('administratif')); await dossierOuvert(p);
  await shot(p, '06-secretariat-instruction-reserves');
  // n° 10 : après décision, les boutons de la pièce disparaissent ; la suivante devient la première.
  await p.getByRole('button', { name: 'Accepter' }).nth(0).click(); await p.waitForTimeout(1200);
  await p.getByRole('button', { name: 'Accepter' }).nth(0).click(); await p.waitForTimeout(1200);
  await p.getByRole('button', { name: 'Passer en instruction' }).click().catch(() => {}); await p.waitForTimeout(1200);
  await shot(p, '07-secretariat-pieces-acceptees'); await b.close();

  ({ b, p } = await ouvrir()); await connecter(p, persona('direction')); await dossierOuvert(p);
  await p.getByLabel('Décision', { exact: true }).selectOption('admis');
  await p.getByRole('button', { name: 'Enregistrer la décision' }).click(); await p.waitForTimeout(1500);
  await shot(p, '08-direction-admis');
  await p.goto(`${BASE}/web/referentiel/annees`, { waitUntil: 'networkidle' }); await p.waitForTimeout(2000);
  await p.getByRole('button', { name: /Préparer l'année suivante de 2026-2027/ }).click(); await p.waitForTimeout(500);
  await shot(p, '09-direction-preparer-annee');
  await p.getByRole('button', { name: 'Préparer', exact: true }).click(); await p.waitForTimeout(2500);
  await shot(p, '10-direction-annee-preparee');
  await p.goto(`${BASE}/web`, { waitUntil: 'networkidle' }); await p.waitForTimeout(3000);
  await shot(p, '11a-direction-accueil-protections-a-confirmer'); // n° 4 : tableau de bord
  await p.goto(`${BASE}/web/direction/protections`, { waitUntil: 'networkidle' }); await p.waitForTimeout(2500);
  await shot(p, '11-direction-protections'); await b.close();

  ({ b, p } = await ouvrir()); await connecter(p, persona('administratif')); await dossierOuvert(p);
  await shot(p, '12-secretariat-validation');
  await p.locator('#ins-val-verifie').check();
  await p.getByRole('button', { name: "Valider l'inscription" }).click(); await p.waitForTimeout(2500);
  await shot(p, '13-secretariat-inscription-validee'); await b.close();

  // ---- 4. Mère : invitation, fiche ---------------------------------------------------------------
  await new Promise(r => setTimeout(r, 12000)); // tâches de fond : invitation de la mère
  const tok = 'd1'.repeat(32);
  sql(`update ent_inscription_invitation set jeton_hash=encode(sha256('${tok}'::bytea),'hex'), statut='envoyee', expire_le=now()+interval '7 days'`);
  ({ b, p } = await ouvrir());
  await p.goto(`${BASE}/web/invitation?jeton=${tok}`, { waitUntil: 'networkidle' }); await p.waitForTimeout(1500);
  await p.getByRole('button', { name: "Accepter l'invitation" }).click(); await p.waitForTimeout(2500);
  await shot(p, '14-mere-invitation-acceptee'); await b.close();
  sql(`update ent_user set password_hash=(select password_hash from ent_user where email like 'parent0-%' limit 1) where email='${MERE}'`);
  const rose = sql(`select id from ent_eleve where prenom='Rose' and nom='Martin'`);
  const fiche = async (email, id, nom, vp) => {
    const o = await ouvrir(vp ? { viewport: { width: 375, height: 812 } } : {});
    await connecter(o.p, email);
    await o.p.goto(`${BASE}/web/eleves/${id}`, { waitUntil: 'networkidle' }); await o.p.waitForTimeout(3000);
    log('==', nom, '\n' + (await o.p.locator('main').innerText()).slice(0, 900).replace(/\n+/g, ' | '));
    await shot(o.p, nom); await o.b.close();
  };
  await fiche(MERE, rose, '15-mere-fiche-rose-pere-masque');
  await fiche(MERE, rose, '15b-mere-fiche-rose-375px', true); // n° 3 (scolarité, INE) et n° 8 (cartes)
  await fiche(persona('direction'), rose, '16-direction-fiche-rose');
  await fiche(persona('administratif'), rose, '17-secretariat-fiche-rose');
  await fiche(persona('administratif'), rose, '17b-secretariat-fiche-rose-375px', true);
  const prof5 = l.personnel.find(x => x.label === 'Prof 5').email;
  await fiche(prof5, rose, '18-prof-sans-classe-fiche-rose-refusee');

  // ---- 5. Professeur d'un seul groupe ---------------------------------------------------------------
  const dir = await jeton(persona('direction'));
  const aujourdhui = new Date().toISOString().slice(0, 10);
  const classes = await api(dir, 'GET', `/referentiel/classes?jour=${aujourdhui}`);
  const c4b = classes.find(c => c.code === '4B').id;
  const g = await api(dir, 'POST', '/referentiel/groupes', { code: '4B-ALL', libelle: '4B LV2 allemand', classe_id: c4b });
  const matiere = (await api(dir, 'GET', '/matieres'))[0].id;
  const prof3 = l.personnel.find(x => x.label === 'Prof 3');
  await api(dir, 'POST', '/referentiel/services', { prof_user_id: prof3.user_id, groupe_id: g.id, matiere_id: matiere });
  const annee = (await api(dir, 'GET', '/referentiel/annees-statut')).find(a => a.courante).id;
  for (const pr of ['Harry', 'Hermione'])
    await api(dir, 'POST', `/eleves/${l.eleves.find(e => e.prenom === pr).id}/scolarite/options`, { annee_id: annee, ajouter: [g.id], date_effet: aujourdhui });
  await fiche(prof3.email, l.eleves.find(e => e.prenom === 'Harry').id, '19-prof-groupe-fiche-harry');
  await fiche(prof3.email, l.eleves.find(e => e.prenom === 'Ginny').id, '20-prof-groupe-fiche-ginny-refusee');
  log('démo terminée');
})();
