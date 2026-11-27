import { test } from "node:test";
import assert from "node:assert/strict";
import {
  appliquerDecisionDirection,
  appliquerReponseParent,
  compterStatuts,
  reserverRappelDirection,
  parisStamp,
  peutCreer,
  peutDeciderDirection,
  peutGerer,
  peutVoir,
  pointVeilleDu,
  relanceDue,
  reponsesCompletes,
  sortieIdDuJeton,
  statutEleve,
  validerSaisie,
  vueSortie,
  type Acteur,
  type SortieClasse,
  type SortieEleve,
} from "./sorties-classe";
import { groupeDirection } from "./sorties-classe-direction";
import { flattenCollectionRecord, inflateFromAttrs } from "./ent-attr-codec";
import { getIntranetModuleById, rolesAllowModule, canAccessIntranetPath } from "./intranet-modules";
import { PROXY_PUBLIC_ROUTE_MATCHERS } from "./public-routes";

const AVANT = new Date("2026-12-01T08:00:00Z");

function eleve(id: string, patch: Partial<SortieEleve> = {}): SortieEleve {
  return {
    studentId: id,
    nom: id.toUpperCase(),
    prenom: "Test",
    classe: "2nde A",
    etablissement: "Lycée",
    token: `jeton-${id}`,
    emails: [`parent-${id}@example.test`],
    parentStatus: "pending",
    sentAt: "2026-12-01T08:00:00.000Z",
    ...patch,
  };
}

function sortie(patch: Partial<SortieClasse> = {}): SortieClasse {
  return {
    id: "sc_abcdefgh1234",
    lieu: "Cinéma",
    date: "2026-12-10",
    heureDepart: "18:00",
    heureRetour: "21:30",
    accompagnateurAller: "A. Dupont",
    accompagnateurRetour: "B. Martin",
    createdBy: { userId: "prof-1", name: "Prof Un", email: "prof@example.test" },
    createdAt: "2026-12-01T08:00:00.000Z",
    updatedAt: "2026-12-01T08:00:00.000Z",
    directions: [{ etablissement: "Lycée", niveau: "lycee", token: "jeton-dir", status: "pending" }],
    eleves: [eleve("a"), eleve("b")],
    ...patch,
  };
}

const acteur = (patch: Partial<Acteur>): Acteur => ({
  userId: "x",
  email: "x@example.test",
  roles: [],
  isAdmin: false,
  ...patch,
});

test("un élève n'est autorisé que si le parent ET la direction ont dit oui", () => {
  const s = sortie();
  assert.equal(statutEleve(s, s.eleves[0]!), "attente_parent");
  assert.ok(appliquerReponseParent(s, "a", { decision: "authorized", source: "lien", par: "P" }, AVANT).ok);
  assert.equal(statutEleve(s, s.eleves[0]!), "attente_direction");
  assert.ok(appliquerDecisionDirection(s, "Lycée", { status: "approved", par: "D" }, AVANT).ok);
  assert.equal(statutEleve(s, s.eleves[0]!), "autorise");
  assert.equal(statutEleve(s, s.eleves[1]!), "attente_parent");
});

test("le refus d'un parent ne touche pas les autres élèves", () => {
  const s = sortie();
  appliquerDecisionDirection(s, "Lycée", { status: "approved", par: "D" }, AVANT);
  appliquerReponseParent(s, "a", { decision: "refused", source: "lien", par: "P" }, AVANT);
  appliquerReponseParent(s, "b", { decision: "authorized", source: "lien", par: "P" }, AVANT);
  assert.deepEqual(compterStatuts(s), { total: 2, autorises: 1, refuses: 1, attenteParent: 0, attenteDirection: 0 });
  assert.ok(reponsesCompletes(s));
});

test("le refus de la direction l'emporte ; annulation = plus personne n'est autorisé", () => {
  const s = sortie();
  appliquerReponseParent(s, "a", { decision: "authorized", source: "lien", par: "P" }, AVANT);
  appliquerDecisionDirection(s, "Lycée", { status: "refused", par: "D" }, AVANT);
  assert.equal(statutEleve(s, s.eleves[0]!), "refus_direction");
  assert.ok(reponsesCompletes(s));
  s.cancelledAt = AVANT.toISOString();
  assert.equal(compterStatuts(s).autorises, 0);
});

test("la première réponse compte ; seul un retrait manuel d'accord est possible", () => {
  const s = sortie();
  appliquerReponseParent(s, "a", { decision: "refused", source: "lien", par: "P1" }, AVANT);
  assert.equal(
    appliquerReponseParent(s, "a", { decision: "authorized", source: "lien", par: "P2" }, AVANT).ok,
    false,
  );
  assert.equal(
    appliquerReponseParent(s, "a", { decision: "authorized", source: "manuel", par: "Prof", note: "appel" }, AVANT).ok,
    false,
  );
  appliquerReponseParent(s, "b", { decision: "authorized", source: "lien", par: "P" }, AVANT);
  assert.ok(
    appliquerReponseParent(s, "b", { decision: "refused", source: "manuel", par: "Prof", note: "appel" }, AVANT).ok,
  );
});

test("saisie manuelle sans précision refusée ; aucune réponse après le départ", () => {
  const s = sortie();
  assert.equal(
    appliquerReponseParent(s, "a", { decision: "authorized", source: "manuel", par: "Prof" }, AVANT).ok,
    false,
  );
  // 18 h à Paris en décembre = 17 h UTC.
  const apres = new Date("2026-12-10T17:00:00Z");
  assert.equal(appliquerReponseParent(s, "a", { decision: "authorized", source: "lien", par: "P" }, apres).ok, false);
  assert.equal(appliquerDecisionDirection(s, "Lycée", { status: "approved", par: "D" }, apres).ok, false);
  assert.equal(parisStamp(apres), "2026-12-10T18:00");
});

test("relance : à partir de J-2, une seule fois, jamais le jour du premier envoi", () => {
  const s = sortie();
  const e = s.eleves[0]!;
  assert.equal(relanceDue(s, e, new Date("2026-12-07T09:00:00Z")), false);
  assert.equal(relanceDue(s, e, new Date("2026-12-08T09:00:00Z")), true);
  const j1 = new Date("2026-12-09T09:00:00Z");
  assert.equal(relanceDue(s, { ...e, relanceAutoAt: "2026-12-08T09:00:00Z" }, j1), false);
  assert.equal(relanceDue(s, { ...e, sentAt: "2026-12-09T08:00:00Z" }, j1), false);
  // Un rappel manuel ancien ne supprime pas le rappel de J-2.
  assert.equal(relanceDue(s, { ...e, remindedAt: "2026-12-02T09:00:00Z" }, j1), true);
  // Invitation jamais partie : nouvel essai.
  assert.equal(relanceDue(s, { ...e, sentAt: undefined }, new Date("2026-12-01T10:00:00Z")), true);
});

test("point final : la veille à midi, ou 3 h après une création tardive", () => {
  const s = sortie();
  assert.equal(pointVeilleDu(s, new Date("2026-12-09T10:59:00Z")), false);
  assert.equal(pointVeilleDu(s, new Date("2026-12-09T11:00:00Z")), true);
  const tardive = sortie({ createdAt: "2026-12-10T07:00:00.000Z" });
  assert.equal(pointVeilleDu(tardive, new Date("2026-12-10T08:00:00Z")), false);
  assert.equal(pointVeilleDu(tardive, new Date("2026-12-10T10:00:00Z")), true);
  assert.equal(pointVeilleDu(sortie({ pointVeilleAt: "x" }), new Date("2026-12-09T12:00:00Z")), false);
  // Départ proche : on n'attend pas 3 h ; après le départ : plus de point.
  assert.equal(pointVeilleDu(tardive, new Date("2026-12-10T08:00:00Z")), false);
  assert.equal(pointVeilleDu(sortie({ createdAt: "2026-12-10T15:00:00.000Z" }), new Date("2026-12-10T15:30:00Z")), true);
  assert.equal(pointVeilleDu(s, new Date("2026-12-10T17:30:00Z")), false);
});

test("la direction est choisie selon le niveau de l'élève, pas selon un libellé mixte", () => {
  const troisieme = groupeDirection({ etablissement: "Collège-Lycée", classe: "3e A" }, []);
  const seconde = groupeDirection({ etablissement: "Collège-Lycée", classe: "2nde B" }, []);
  assert.equal(troisieme.niveau, "college");
  assert.equal(seconde.niveau, "lycee");
  assert.notEqual(troisieme.groupe, seconde.groupe);
  const inconnu = groupeDirection({ etablissement: "", classe: "Sans classe" }, []);
  assert.equal(inconnu.niveau, null);
  const direction = { etablissement: inconnu.groupe, niveau: null, token: "t", status: "pending" as const };
  assert.ok(!peutDeciderDirection(acteur({ roles: ["direction_college"] }), direction));
  assert.ok(peutDeciderDirection(acteur({ roles: ["direction"] }), direction));
});

test("validation de la saisie", () => {
  const ok = {
    lieu: "Musée",
    date: "2026-12-10",
    heureDepart: "14:00",
    heureRetour: "17:00",
    accompagnateurAller: "A",
    accompagnateurRetour: "A",
    studentIds: ["1", "1", "2"],
  };
  const res = validerSaisie(ok, AVANT);
  assert.ok(res.ok && res.saisie.studentIds.length === 2);
  for (const patch of [
    { lieu: " " },
    { date: "2026-02-31" },
    { date: "2026-11-01" },
    { heureRetour: "13:00" },
    { accompagnateurRetour: "" },
    { studentIds: [] },
  ]) {
    assert.equal(validerSaisie({ ...ok, ...patch }, AVANT).ok, false, JSON.stringify(patch));
  }
});

test("droits : organisateur et admin gèrent, la direction concernée valide, les autres ne voient rien", () => {
  const s = sortie();
  const organisateur = acteur({ userId: "prof-1", roles: ["professeur"] });
  const autreProf = acteur({ userId: "prof-2", roles: ["professeur"] });
  const admin = acteur({ isAdmin: true });
  const dirLycee = acteur({ roles: ["direction_lycee"] });
  const dirCollege = acteur({ roles: ["direction_college"] });
  assert.ok(peutGerer(organisateur, s) && peutGerer(admin, s));
  assert.ok(!peutGerer(autreProf, s) && !peutGerer(dirLycee, s));
  assert.ok(!peutVoir(autreProf, s) && !peutVoir(dirCollege, s) && peutVoir(dirLycee, s));
  assert.ok(peutDeciderDirection(dirLycee, s.directions[0]!));
  assert.ok(!peutDeciderDirection(organisateur, s.directions[0]!));
  assert.ok(peutCreer(autreProf) && peutCreer(admin) && !peutCreer(dirLycee));
});

test("la vue envoyée au navigateur ne contient ni jeton, ni e-mail, ni adresse IP", () => {
  const s = sortie({ eleves: [eleve("a", { responseIp: "203.0.113.7", note: "privé" })] });
  const json = JSON.stringify(vueSortie(s, null));
  for (const secret of ["jeton-a", "jeton-dir", "parent-a@example.test", "203.0.113.7", "privé", "prof@example.test"]) {
    assert.ok(!json.includes(secret), secret);
  }
});

test("jeton : seul le format attendu désigne une sortie", () => {
  assert.equal(sortieIdDuJeton(`sc_abcdefgh1234.${"A".repeat(43)}`), "sc_abcdefgh1234");
  assert.equal(sortieIdDuJeton("sc_abcdefgh1234"), null);
  assert.equal(sortieIdDuJeton(`../x.${"A".repeat(43)}`), null);
});

test("une sortie survit à l'enregistrement en base (codec par attributs)", () => {
  const s = sortie({ eleves: [eleve("a", { emails: [] }), eleve("b")] });
  const relu = inflateFromAttrs(flattenCollectionRecord(s as unknown as Record<string, unknown>)) as unknown as SortieClasse;
  assert.equal(relu.eleves.length, 2);
  assert.deepEqual(relu.eleves[0]!.emails, []);
  assert.deepEqual(relu.eleves[1]!.emails, ["parent-b@example.test"]);
  assert.equal(relu.directions[0]!.status, "pending");
  assert.equal(relu.cancelledAt, undefined);
});

test("module : professeurs et direction y accèdent même avec une ancienne matrice de droits", () => {
  const mod = getIntranetModuleById("sorties-classe")!;
  const matrice = { byRole: { professeur: { modules: ["travels"] } } };
  assert.ok(rolesAllowModule(["professeur"], mod, false, matrice));
  assert.ok(rolesAllowModule(["direction_lycee"], mod, false));
  assert.ok(!rolesAllowModule(["surveillant"], mod, false));
  assert.ok(!rolesAllowModule(["parent"], mod, false));
});

/** Même règle de correspondance que le proxy : « (.*) » = n'importe quelle suite. */
function isPublicPath(path: string): boolean {
  return (PROXY_PUBLIC_ROUTE_MATCHERS as readonly string[]).some((pattern) => {
    const [base, wildcard] = [pattern.replace("(.*)", ""), pattern.endsWith("(.*)")];
    return wildcard ? path.startsWith(base) : path === base;
  });
}

test("les pages de réponse sont publiques, l'espace des professeurs ne l'est pas", () => {
  assert.ok(isPublicPath("/internat/autorisation/classe"));
  assert.ok(isPublicPath("/api/internat/outings/decision/classe"));
  assert.ok(!isPublicPath("/api/sorties-classe"));
  assert.ok(!isPublicPath("/api/internat/outings/classe"));
  assert.ok(!isPublicPath("/autorisations-sortie"));
});

test("professeur : la page Internat seulement, ni ses sous-pages ni les données de l'internat", () => {
  const peut = (path: string, roles = ["professeur"]) => canAccessIntranetPath(path, roles, false);
  assert.ok(peut("/gestion-internat"));
  assert.ok(peut("/gestion-internat?tab=internes"));
  assert.ok(peut("/api/sorties-classe"));
  for (const path of [
    "/gestion-internat/appel-soir",
    "/gestion-internat/affectations",
    "/api/internat",
    "/api/internat/students",
    "/api/internat/rooms",
    "/api/internat/incidents",
    "/api/internat/stats",
    "/api/internat/outings",
    "/api/internat/outings/classe",
  ]) {
    assert.ok(!peut(path), path);
  }
  assert.ok(peut("/gestion-internat/appel-soir", ["surveillant"]));
  assert.ok(!peut("/gestion-internat", ["parent"]));
});

test("rappel à la direction : sans réponse seulement, une fois par heure ; compteurs séparés", () => {
  const s = sortie({ eleves: [eleve("a", { parentStatus: "authorized" }), eleve("b")] });
  s.directions[0]!.status = "pending";
  const nom = s.directions[0]!.etablissement;
  for (const e of s.eleves) e.etablissement = nom;
  assert.deepEqual(compterStatuts(s), { total: 2, autorises: 0, refuses: 0, attenteParent: 1, attenteDirection: 1 });
  const now = new Date(`${s.date}T00:00:00Z`);
  now.setUTCDate(now.getUTCDate() - 3);
  assert.ok(reserverRappelDirection(s, nom, now).ok);
  assert.ok(!reserverRappelDirection(s, nom, new Date(now.getTime() + 10 * 60_000)).ok);
  assert.ok(reserverRappelDirection(s, nom, new Date(now.getTime() + 61 * 60_000)).ok);
  s.directions[0]!.status = "approved";
  assert.ok(!reserverRappelDirection(s, nom, new Date(now.getTime() + 180 * 60_000)).ok);
});
