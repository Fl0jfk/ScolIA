import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  isAcceptableTravelCoverSize,
  parseTravelCoverVisionPick,
  sortCommonsPagesByIndex,
  travelCoverSourceOrder,
  wikimediaThumbAtWidth,
} from "./travels-cover-pick";

describe("travels-cover-pick", () => {
  it("met Wikipedia en premier pour un lieu, les banques d'images pour une activité", () => {
    assert.equal(travelCoverSourceOrder("place", true)[0], "wikipedia-fr");
    assert.equal(travelCoverSourceOrder("activity", true)[0], "unsplash");
  });

  it("retire Wikipedia quand la requête est à risque (drone…)", () => {
    const order = travelCoverSourceOrder("place", false);
    assert.ok(!order.includes("wikipedia-fr"));
    assert.ok(!order.includes("wikipedia-en"));
    assert.equal(order[0], "commons");
  });

  it("refuse les images minuscules ou très en hauteur", () => {
    assert.equal(isAcceptableTravelCoverSize(320, 200), false);
    assert.equal(isAcceptableTravelCoverSize(1000, 2200), false);
    assert.equal(isAcceptableTravelCoverSize(2900, 5367), true); // Tour Eiffel (Wikipedia)
    assert.equal(isAcceptableTravelCoverSize(1280, 853), true);
    assert.equal(isAcceptableTravelCoverSize(1000, 1200), true);
    assert.equal(isAcceptableTravelCoverSize(undefined, undefined), true);
  });

  it("remet les résultats Commons dans l'ordre de pertinence", () => {
    const sorted = sortCommonsPagesByIndex([
      { id: "a", index: 3 },
      { id: "b", index: 1 },
      { id: "c" },
      { id: "d", index: 2 },
    ]);
    assert.deepEqual(sorted.map((p) => p.id), ["b", "d", "a", "c"]);
  });

  it("fabrique une miniature Wikimedia à la bonne largeur", () => {
    assert.equal(
      wikimediaThumbAtWidth(
        "https://upload.wikimedia.org/wikipedia/commons/a/a8/Tour_Eiffel.jpg",
        500,
      ),
      "https://upload.wikimedia.org/wikipedia/commons/thumb/a/a8/Tour_Eiffel.jpg/500px-Tour_Eiffel.jpg",
    );
    assert.equal(
      wikimediaThumbAtWidth(
        "https://upload.wikimedia.org/wikipedia/commons/thumb/a/a8/Tour_Eiffel.jpg/1280px-Tour_Eiffel.jpg?utm=x",
        500,
      ),
      "https://upload.wikimedia.org/wikipedia/commons/thumb/a/a8/Tour_Eiffel.jpg/500px-Tour_Eiffel.jpg",
    );
    assert.equal(wikimediaThumbAtWidth("https://images.unsplash.com/photo-1", 500), null);
  });

  it("lit la réponse de la vérification visuelle", () => {
    assert.equal(parseTravelCoverVisionPick('{"best": 2, "why": "vrai lieu"}', 4), 1);
    assert.equal(parseTravelCoverVisionPick('{"best": "NONE"}', 4), null);
    assert.equal(parseTravelCoverVisionPick('{"best": null}', 4), null);
    assert.equal(parseTravelCoverVisionPick("3", 4), 2);
    assert.equal(parseTravelCoverVisionPick("NONE", 4), null);
    assert.equal(parseTravelCoverVisionPick('{"best": 9}', 4), undefined);
    assert.equal(parseTravelCoverVisionPick("", 4), undefined);
  });
});
