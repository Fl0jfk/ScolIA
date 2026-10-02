import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  buildTravelPlaceSearchQuery,
  buildTravelWebSearchQueries,
  normalizeTravelImageKey,
  rankTravelCatalogCandidates,
  scoreTravelCatalogMatch,
  tokenizeTravelPlaceQuery,
} from "./travels-image-catalog-helpers";

describe("travels-image-catalog-helpers", () => {
  it("normalise les clés lieu", () => {
    assert.equal(normalizeTravelImageKey("Disneyland Paris"), "disneylandparis");
    assert.equal(normalizeTravelImageKey("Château de Chambord"), "chateaudechambord");
  });

  it("tokenise en ignorant les stop words", () => {
    const tokens = tokenizeTravelPlaceQuery("Sortie Disneyland", "Disneyland Paris");
    assert.ok(tokens.includes("disneyland"));
    assert.ok(tokens.includes("paris"));
    assert.ok(!tokens.includes("sortie"));
  });

  it("classe Disneyland devant Astérix", () => {
    const catalog = [
      {
        id: "Disneyland Paris",
        label: "Disneyland Paris",
        url: "https://example.com/disney.jpg",
        keywords: "disney, disneyland, paris, parc",
        normalizeKey: "disneylandparis",
      },
      {
        id: "Parc Astérix",
        label: "Parc Astérix",
        url: "https://example.com/asterix.jpg",
        keywords: "asterix, paris, parc",
        normalizeKey: "parcasterix",
      },
    ];
    const ranked = rankTravelCatalogCandidates(
      catalog,
      "Sortie CM2",
      "Disneyland Paris",
    );
    assert.equal(ranked[0]?.id, "Disneyland Paris");
    assert.ok(
      scoreTravelCatalogMatch(
        ranked[0]!,
        tokenizeTravelPlaceQuery("Disneyland Paris"),
      ) >= 6,
    );
  });

  it("construit une requête lieu propre", () => {
    assert.equal(
      buildTravelPlaceSearchQuery("Sortie pédagogique", "Château de Chambord"),
      "Château de Chambord",
    );
    assert.equal(
      buildTravelPlaceSearchQuery("Joueur surf", ""),
      "surf",
    );
  });

  it("priorise le thème surf avant le lieu pour le web", () => {
    const queries = buildTravelWebSearchQueries("Joueur surf", "Rouen");
    assert.equal(queries[0], "surf");
    assert.ok(queries.includes("surf"));
    assert.ok(!queries.includes("joueur"));
    assert.ok(queries.some((q) => /rouen/i.test(q)));
  });

  it("ne matche pas Beaux-Arts pour un thème surf", () => {
    const beauxArts = {
      id: "Musée des Beaux Arts de Rouen",
      label: "Musée des Beaux Arts de Rouen",
      url: "https://example.com/mba.jpg",
      keywords: "musée, beaux-arts, rouen",
      normalizeKey: "museedesbeauxartsderouen",
    };
    const tokens = tokenizeTravelPlaceQuery("Joueur surf", "Surf");
    assert.equal(scoreTravelCatalogMatch(beauxArts, tokens), 0);
    const ranked = rankTravelCatalogCandidates(
      [beauxArts],
      "Joueur surf",
      "Surf",
    );
    assert.equal(ranked.length, 0);
  });
});
