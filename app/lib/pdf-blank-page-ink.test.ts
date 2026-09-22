import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  computeInkStatsFromRgba,
  isBlankPageFromInkStats,
} from "./pdf-blank-page-ink";

function fillRgba(
  width: number,
  height: number,
  paint: (x: number, y: number) => [number, number, number],
): Uint8ClampedArray {
  const data = new Uint8ClampedArray(width * height * 4);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const [r, g, b] = paint(x, y);
      const i = (y * width + x) * 4;
      data[i] = r;
      data[i + 1] = g;
      data[i + 2] = b;
      data[i + 3] = 255;
    }
  }
  return data;
}

describe("pdf-blank-page-ink", () => {
  it("détecte une page quasi blanche (bruit scanner léger)", () => {
    const w = 200;
    const h = 280;
    let dust = 0;
    const data = fillRgba(w, h, (x, y) => {
      // Quelques grains isolés (luma 235) — sous le seuil soft 0.1 %
      if (dust < 8 && x > 40 && y > 40 && (x + y) % 311 === 0) {
        dust++;
        return [235, 235, 235];
      }
      return [252, 252, 252];
    });
    const stats = computeInkStatsFromRgba(data, w, h);
    assert.ok(stats.softInkRatio < 0.001);
    assert.equal(isBlankPageFromInkStats(stats), true);
  });

  it("garde une page avec une petite zone d’encre (tampon / case)", () => {
    const w = 200;
    const h = 280;
    const data = fillRgba(w, h, (x, y) => {
      // Carré sombre ~20×20 au centre (~400 px ≈ 0.7 % de la zone utile)
      if (x >= 90 && x < 110 && y >= 130 && y < 150) return [20, 20, 20];
      return [252, 252, 252];
    });
    const stats = computeInkStatsFromRgba(data, w, h);
    assert.equal(isBlankPageFromInkStats(stats), false);
  });

  it("ignore le bruit en marge (perforations)", () => {
    const w = 200;
    const h = 280;
    const data = fillRgba(w, h, (x, y) => {
      if (x < 6 || x > w - 7) return [10, 10, 10];
      return [254, 254, 254];
    });
    const stats = computeInkStatsFromRgba(data, w, h);
    assert.equal(isBlankPageFromInkStats(stats), true);
  });
});
