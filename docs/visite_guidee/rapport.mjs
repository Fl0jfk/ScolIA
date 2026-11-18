// Génère VISITE.md (parties mécaniques) à partir de 2026-11/resultats.json, de la carte des routes (routes.json)
// et des commentaires rédigés à la main (commentaires.json). Usage : node rapport.mjs
import fs from 'node:fs';
import path from 'node:path';
const dir = new URL('./2026-11/', import.meta.url).pathname;
const R = JSON.parse(fs.readFileSync(dir + 'resultats.json', 'utf8'));
const routes = JSON.parse(fs.readFileSync(new URL('./routes.json', import.meta.url), 'utf8'));
const C = JSON.parse(fs.readFileSync(new URL('./commentaires.json', import.meta.url), 'utf8'));
const TITRES = { '00-accueil': 'Accueil, à faire, recherche', '01-eleve': 'Élèves et classes', '02-inscriptions': 'Inscriptions et réinscriptions', '03-assiduite': 'Vie scolaire : appel, absences, assiduité, discipline',
  '04-absences-personnels': 'Absences des personnels, remplacements, heures', '05-edt': 'Emploi du temps', '06-cahier': 'Cahier de textes et travail à faire', '07-evaluations': 'Évaluations, notes, bulletins',
  '08-messagerie': 'Messagerie', '09-agenda': 'Agenda, salles, rendez-vous', '10-voyages': 'Sorties et voyages', '11-demandes': 'Demandes et files de traitement', '12-formulaires': 'Formulaires, consentements, campagnes',
  '13-finances': 'Finances', '14-sante-accompagnement': 'Santé, aménagements, accompagnement', '15-administration': 'Administration : référentiel, rôles, sécurité', '16-internat': 'Internat (sans écran)', '17-documents': 'Documents (sans écran)' };
const ROLES = { direction: 'Direction', administratif: 'Secrétariat', cpe: 'CPE / vie scolaire', accueil: 'Accueil', prof: 'Professeur', prof_principal: 'Professeur principal', infirmerie: 'Infirmerie', compta: 'Comptabilité', psy: 'Psychologue', personnel: 'Personnel (AED/agent)', parent: 'Parent', parent2: 'Parent (2 enfants)', eleve_mineur: 'Élève mineur', eleve_majeur: 'Élève majeur', admin: 'Administrateur' };
const rel = (f) => `2026-11/${f}`;
const existe = (f) => fs.existsSync(dir + f);
const mobile = (f) => { const m = f.match(/^(.*\/)([a-z-]+)-(\d+)-(.*)\.png$/); if (!m) return null; const g = `${m[1]}${m[2]}-${String(+m[3] + 1).padStart(2, '0')}-${m[4]}-375.png`; return existe(g) ? g : null; };
const doms = [...new Set([...R.ecrans.map((e) => e.domaine), ...R.actions.map((a) => a.domaine), ...Object.keys(routes)])].filter((d) => d !== '99-autre').sort();
for (const a of R.actions) for (const k of C.corrections ?? []) if (a.description.includes(k.contient)) { a.statut = k.statut; a.detail = k.detail; }
const bruit = /^\d+ résultats?\.?$/;
let md = '';
const ecransUniques = (d) => { const m = new Map(); for (const e of R.ecrans.filter((x) => x.domaine === d)) { const k = e.chemin.replace(/[0-9a-f]{8}-[0-9a-f-]{27}/g, ':id'); (m.get(k) ?? m.set(k, []).get(k)).push(e); } return m; };

md += `# Visite guidée de l'ENT ScolIA : état au 18 novembre 2026\n\n`;
md += C.intro.join('\n\n') + '\n\n';
md += `## Chiffres de la visite\n\n- **${R.ecrans.length} écrans** photographiés (1 280 px), dont ${new Set(R.ecrans.map((e) => e.chemin.replace(/[0-9a-f]{8}-[0-9a-f-]{27}/g, ':id'))).size} chemins distincts ; une capture à 375 px pour les écrans principaux.\n- **${R.actions.length} actions** jouées (saisies, validations, envois, créations), chacune avec capture et résultat réel.\n- Captures par domaine : ${Object.entries(R.captures).sort().map(([d, n]) => `${d.slice(3)} ${n}`).join(' · ')}.\n- Dossier : \`docs/visite_guidee/2026-11/<domaine>/\` ; nom : \`<rôle>-<n°>-<écran ou action>[-375].png\` (le numéro suit l'ordre de la visite du rôle).\n\n`;

md += `## Retours de Florian pendant la visite (18-11)\n\n` + C.retours.join('\n\n') + '\n\n';
md += `### Entrées de la barre de gauche, par rôle (constat)\n\n| Rôle | Entrées | Exemples |\n|---|---|---|\n`;
for (const [k, l] of Object.entries(R.menus)) md += `| ${ROLES[k] ?? k} | **${l.length}** | ${l.slice(0, 5).map((x) => x[1]).join(', ')}… |\n`;
md += `\nAucun regroupement : la liste est plate, dans l'ordre d'ajout des modules. Le seul signal chiffré visible dans la barre est le compteur de messagerie.\n\n`;

md += `## Domaine par domaine\n\nLégende des signaux : ⚠ = relevé automatiquement (écran presque vide, « pas encore disponible », alerte, refus du serveur, 5xx, écran resté en chargement) ; chaque ⚠ est ensuite confirmé ou écarté à la main dans « Absent ou mince ».\n\n`;
for (const d of doms) {
  md += `### ${d.slice(0, 2)}. ${TITRES[d] ?? d}\n\n`;
  const U = ecransUniques(d);
  if (U.size) {
    md += `**Écrans visités** (${U.size})\n\n| Écran | Titre | Rôles qui y accèdent | Captures | Signaux |\n|---|---|---|---|---|\n`;
    for (const [chemin, l] of U) {
      const roles = [...new Set(l.map((e) => e.role))];
      const premier = l[0]; const m = mobile(premier.capture);
      const sig = [...new Set(l.flatMap((e) => e.signaux))].filter((x) => !/Failed to load resource|^console/.test(x)).map((x) => x.replace(/\s+/g, ' ').slice(0, 90));
      const sig403 = l.some((e) => e.signaux.some((x) => /403/.test(x)));
      md += `| \`${chemin.replace('/web', '') || '/'}\` | ${premier.titre.replace(/\|/g, '/').slice(0, 60)} | ${roles.map((r) => ROLES[r] ?? r).join(', ')} | [${premier.role}](${rel(premier.capture)})${m ? ` · [375 px](${rel(m)})` : ''} | ${sig.length || sig403 ? '⚠ ' + sig.concat(sig403 ? ['appel serveur refusé (403) pendant le chargement'] : []).join(' ; ') : ''} |\n`;
    }
    md += '\n';
  }
  const A = R.actions.filter((a) => a.domaine === d);
  if (A.length) {
    md += `**Actions jouées** (${A.length})\n\n| Rôle | Action | Résultat | Retour de l'écran | Capture |\n|---|---|---|---|---|\n`;
    for (const a of A) md += `| ${ROLES[a.role] ?? a.role} | ${a.description} | ${a.statut === 'ok' ? 'OK' : '**' + a.statut + '**'} | ${(a.detail || a.retour.filter((x) => !bruit.test(x)).join(' / ')).replace(/\s+/g, ' ').replace(/\|/g, '/').slice(0, 150)} | [image](${rel(a.capture)}) |\n`;
    md += '\n';
  }
  const c = C.domaines[d];
  if (c) {
    md += `**Ce que chaque rôle peut faire (constaté)**\n\n${c.roles.map((x) => '- ' + x).join('\n')}\n\n`;
    md += `**Absent ou visiblement mince**\n\n${c.manques.map((x) => '- ' + x).join('\n')}\n\n`;
  }
  const r = routes[d];
  if (r) md += `**Backend sans écran** : ${r.sans_ecran} routes de \`ent-api\` sur ${r.total} ne sont appelées par aucun écran Leptos (ex. : ${r.exemples.slice(0, 6).map((x) => '`' + x + '`').join(', ')}). ${C.backend?.[d] ?? ''}\n\n`;
}

md += `## Tableau de synthèse\n\n| Domaine | Écrans existants | Actions jouables (jouées ici) | Manques visibles | Backend sans écran |\n|---|---|---|---|---|\n`;
for (const d of doms) {
  const U = ecransUniques(d); const A = R.actions.filter((a) => a.domaine === d); const r = routes[d]; const c = C.domaines[d];
  md += `| ${TITRES[d] ?? d} | ${U.size} | ${A.length ? `${A.filter((a) => a.statut === 'ok').length} sur ${A.length}` : '0'} | ${c ? c.synthese : ''} | ${r ? `${r.sans_ecran} / ${r.total}` : ''} |\n`;
}
md += `\n_Aucun pourcentage : c'est Florian qui juge._ « Backend sans écran » = routes de \`ent-api\` sans appel correspondant dans \`ent-web-api\` (comparaison automatique des chemins ; certaines routes servent l'ancienne interface, les applications mobiles ou des tâches internes, voir la méthode).\n\n`;
md += C.fin.join('\n\n') + '\n';
fs.writeFileSync(new URL('./VISITE.md', import.meta.url), md);
console.log('VISITE.md', md.length, 'octets');
