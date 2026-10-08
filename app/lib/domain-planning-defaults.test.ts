import assert from "node:assert/strict";
import test from "node:test";
import {
  buildDefaultLyceeSessions,
  buildEmptyLyceeSessions,
  classesForTransversalNiveau,
  DEFAULT_ALL_EVARS_SESSIONS,
  DEFAULT_DOMAIN_ID,
  DEFAULT_DOMAIN_PLANNING_DOMAINS,
  DEFAULT_EVARS_LYCEE_DOMAIN_ID,
  DEFAULT_EVARS_LYCEE_SESSIONS,
  DEFAULT_EVARS_SESSIONS,
  ensureLyceeDomainPresent,
  ensureLyceeSessionsPresent,
  hydrateEmptySessionThemes,
  isTransversalNiveau,
} from "./domain-planning-defaults";

test("DEFAULT_EVARS_SESSIONS are attached to the default college domain", () => {
  assert.ok(DEFAULT_EVARS_SESSIONS.length > 0);
  for (const session of DEFAULT_EVARS_SESSIONS) {
    assert.equal(session.domainId, DEFAULT_DOMAIN_ID);
    assert.ok(isTransversalNiveau(session.niveau));
    assert.ok(session.theme.trim().length > 0);
  }
});

test("DEFAULT_EVARS_LYCEE_SESSIONS cover 2nde / 1ère / Tle with official themes", () => {
  assert.equal(DEFAULT_EVARS_LYCEE_SESSIONS.length, 9);
  for (const session of DEFAULT_EVARS_LYCEE_SESSIONS) {
    assert.equal(session.domainId, DEFAULT_EVARS_LYCEE_DOMAIN_ID);
    assert.ok(["2nde", "1ere", "tle"].includes(session.niveau));
    assert.ok(session.theme.trim().length > 0);
    assert.ok([1, 2, 3].includes(session.seanceNumber));
    // Lycée : pas d'association — uniquement choix libre professeurs.
    assert.equal(session.intervenantConstraint, "free");
    assert.equal(session.intervenantLabel, "Au choix des professeurs");
  }
  assert.deepEqual(
    [...new Set(DEFAULT_EVARS_LYCEE_SESSIONS.map((s) => s.niveau))].sort(),
    ["1ere", "2nde", "tle"],
  );
  const byNiveau = Object.fromEntries(
    (["2nde", "1ere", "tle"] as const).map((niveau) => [
      niveau,
      DEFAULT_EVARS_LYCEE_SESSIONS.filter((s) => s.niveau === niveau).map((s) => s.theme),
    ]),
  );
  assert.deepEqual(byNiveau["2nde"], [
    "Image, estime et confiance en soi",
    "Reconnaître et comprendre ses émotions",
    "L'intimité à l'ère des réseaux sociaux",
  ]);
  assert.deepEqual(byNiveau["1ere"], [
    "Plaisir, excès et conduites à risques : faire des choix éclairés",
    "Savoir dire oui ou non : le consentement",
    "Accueillir la diversité",
  ]);
  assert.deepEqual(byNiveau.tle, [
    "Comprendre les enjeux de la pornographie",
    "Vivre une sexualité épanouie ou Développer une relation saine",
    "Ma place dans le monde : oser être soi",
  ]);
});

test("DEFAULT_DOMAIN_PLANNING_DOMAINS includes college and lycée EVARS", () => {
  const ids = DEFAULT_DOMAIN_PLANNING_DOMAINS.map((d) => d.id);
  assert.ok(ids.includes(DEFAULT_DOMAIN_ID));
  assert.ok(ids.includes(DEFAULT_EVARS_LYCEE_DOMAIN_ID));
});

test("DEFAULT_ALL_EVARS_SESSIONS concatenates college and lycée", () => {
  assert.equal(
    DEFAULT_ALL_EVARS_SESSIONS.length,
    DEFAULT_EVARS_SESSIONS.length + DEFAULT_EVARS_LYCEE_SESSIONS.length,
  );
});

test("classesForTransversalNiveau matches college and lycée class codes", () => {
  const classesByPole = {
    COLLÈGE: ["6A", "5B", "3C"],
    LYCÉE: ["2A", "1B", "TA", "TB"],
  };
  assert.deepEqual(classesForTransversalNiveau("6e", classesByPole), ["6A"]);
  assert.deepEqual(classesForTransversalNiveau("2nde", classesByPole), ["2A"]);
  assert.deepEqual(classesForTransversalNiveau("1ere", classesByPole), ["1B"]);
  assert.deepEqual(classesForTransversalNiveau("tle", classesByPole), ["TA", "TB"]);
});

test("buildDefaultLyceeSessions creates a filled 3×3 grid for a domain", () => {
  const sessions = buildDefaultLyceeSessions("evars-lycee");
  assert.equal(sessions.length, 9);
  assert.ok(sessions.every((s) => s.domainId === "evars-lycee"));
  assert.ok(sessions.every((s) => s.theme.trim().length > 0));
  assert.deepEqual(
    [...new Set(sessions.map((s) => s.niveau))].sort(),
    ["1ere", "2nde", "tle"],
  );
});

test("buildEmptyLyceeSessions remains an alias of buildDefaultLyceeSessions", () => {
  assert.deepEqual(buildEmptyLyceeSessions("custom-lycee"), buildDefaultLyceeSessions("custom-lycee"));
});

test("hydrateEmptySessionThemes fills empty lycée grids from the programme", () => {
  const empty = buildDefaultLyceeSessions("custom-lycee").map((s) => ({
    ...s,
    theme: "",
    intervenantLabel: "Au choix des professeurs",
    intervenantConstraint: "free" as const,
    mixte: true,
  }));
  const hydrated = hydrateEmptySessionThemes(empty);
  assert.equal(hydrated[0]?.theme, "Image, estime et confiance en soi");
  assert.equal(hydrated[0]?.intervenantConstraint, "free");
  assert.equal(hydrated[4]?.theme, "Savoir dire oui ou non : le consentement");
  assert.equal(hydrated[4]?.intervenantConstraint, "free");
});

test("hydrateEmptySessionThemes unlocks Association / SVT / psy on lycée sessions", () => {
  const locked = buildDefaultLyceeSessions("custom-lycee").map((s, idx) => {
    if (idx === 1) {
      return {
        ...s,
        intervenantLabel: "Association",
        intervenantConstraint: "fixed_association" as const,
        mixte: false,
      };
    }
    if (idx === 0) {
      return {
        ...s,
        intervenantLabel: "Profs d'SVT",
        intervenantConstraint: "svt_only" as const,
      };
    }
    if (idx === 4) {
      return {
        ...s,
        intervenantLabel: "Psychologue / Infirmière",
        intervenantConstraint: "psy_inf" as const,
        mixte: false,
      };
    }
    return s;
  });
  const hydrated = hydrateEmptySessionThemes(locked);
  assert.ok(hydrated.every((s) => s.intervenantConstraint === "free"));
  assert.ok(hydrated.every((s) => s.intervenantLabel === "Au choix des professeurs"));
});

test("hydrateEmptySessionThemes corrects outdated lycée themes already stored", () => {
  const outdated = buildDefaultLyceeSessions("custom-lycee").map((s) => {
    if (s.niveau === "1ere" && s.seanceNumber === 1) {
      return { ...s, theme: "Plaisir, excès, conduites à risque : faire des choix éclairés" };
    }
    if (s.niveau === "1ere" && s.seanceNumber === 3) {
      return { ...s, theme: "Ma place dans le monde : oser être soi" };
    }
    if (s.niveau === "tle" && s.seanceNumber === 2) {
      return { ...s, theme: "Vivre une sexualité épanouie" };
    }
    if (s.niveau === "tle" && s.seanceNumber === 3) {
      return { ...s, theme: "Être libre d'être soi parmi les autres" };
    }
    return s;
  });
  const hydrated = hydrateEmptySessionThemes(outdated);
  assert.equal(
    hydrated.find((s) => s.niveau === "1ere" && s.seanceNumber === 1)?.theme,
    "Plaisir, excès et conduites à risques : faire des choix éclairés",
  );
  assert.equal(
    hydrated.find((s) => s.niveau === "1ere" && s.seanceNumber === 3)?.theme,
    "Accueillir la diversité",
  );
  assert.equal(
    hydrated.find((s) => s.niveau === "tle" && s.seanceNumber === 2)?.theme,
    "Vivre une sexualité épanouie ou Développer une relation saine",
  );
  assert.equal(
    hydrated.find((s) => s.niveau === "tle" && s.seanceNumber === 3)?.theme,
    "Ma place dans le monde : oser être soi",
  );
});

test("ensureLyceeSessionsPresent appends defaults when only college exists", () => {
  const merged = ensureLyceeSessionsPresent([...DEFAULT_EVARS_SESSIONS]);
  assert.equal(merged.length, DEFAULT_ALL_EVARS_SESSIONS.length);
  assert.ok(merged.some((s) => s.niveau === "2nde" && s.theme.includes("estime")));
});

test("ensureLyceeDomainPresent adds evars-lycee next to college", () => {
  const domains = ensureLyceeDomainPresent([
    {
      id: DEFAULT_DOMAIN_ID,
      name: "EVARS",
      coordinatorExternalUserIds: [],
    },
  ]);
  assert.ok(domains.some((d) => d.id === DEFAULT_EVARS_LYCEE_DOMAIN_ID));
});
