import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  computeParentBlogExpiresAt,
  parisEndOfDayUtc,
  PARENT_BLOG_RETENTION_DAYS,
} from "./travels-parent-blog";

test("rétention : 7 jours", () => {
  assert.equal(PARENT_BLOG_RETENTION_DAYS, 7);
});

test("expiration : fin du séjour + 7 jours à 23:59:59 heure de Paris (été)", () => {
  const d = computeParentBlogExpiresAt({ type: "COMPLEX", data: { endDate: "2027-06-10" } } as never);
  assert.equal(d.toISOString(), "2027-06-17T21:59:59.000Z");
});

test("expiration : heure d'hiver (+01:00)", () => {
  const d = computeParentBlogExpiresAt({ type: "COMPLEX", data: { endDate: "2027-01-10" } } as never);
  assert.equal(d.toISOString(), "2027-01-17T22:59:59.000Z");
});

test("expiration : traversée du changement d'heure (fin mars 2027)", () => {
  // séjour fini le 25/03/2027 (hiver) ; +7 j = 01/04 (été, +02:00)
  const d = computeParentBlogExpiresAt({ type: "COMPLEX", data: { endDate: "2027-03-25" } } as never);
  assert.equal(d.toISOString(), "2027-04-01T21:59:59.000Z");
  assert.equal(parisEndOfDayUtc("2027-03-28").toISOString(), "2027-03-28T21:59:59.000Z");
});

test("suppression d'une publication : cloisonnement voyage + établissement avant toute suppression S3", () => {
  const src = readFileSync(new URL("./travels-parent-blog.ts", import.meta.url), "utf8").replace(/\r\n/g, "\n");
  const start = src.indexOf("export async function deleteParentBlogPost");
  const body = src.slice(start, src.indexOf("export ", start + 10));
  const check = body.indexOf("ownPost");
  const s3 = body.indexOf("deleteObject(");
  assert.ok(check > 0 && s3 > check, "vérif d'appartenance du post avant deleteObject");
  assert.match(body, /travelParentBlogPhoto\.travelId, trip\.id/);
  assert.match(body, /travelParentBlogPhoto\.etablissementId, etablissementId/);
  assert.match(body, /Publication introuvable/);
});

test("délégués : route réservée organisateur/direction + appartenance établissement", () => {
  const src = readFileSync(new URL("../api/travels/parent-blog/delegates/route.ts", import.meta.url), "utf8");
  assert.match(src, /!isOwner && !canSign/);
  assert.match(src, /filterDelegatesInEtablissement/);
});

test("API publique : noindex, no-store, referrer, limitation de débit", () => {
  const src = readFileSync(new URL("../api/travels/public-blog/route.ts", import.meta.url), "utf8");
  assert.match(src, /noindex, nofollow/);
  assert.match(src, /no-store/);
  assert.match(src, /no-referrer/);
  assert.match(src, /publicBlogLimiter\.allow/);
});
