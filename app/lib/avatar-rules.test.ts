import assert from "node:assert/strict";
import { test } from "node:test";
import sharp from "sharp";
import { AVATAR_SIDE, toAvatarJpeg } from "@/app/lib/avatar-image";
import { avatarUrl, canManageAvatarOf, canManageOthersAvatars } from "@/app/lib/avatar-rules";
import { canAccessIntranetPath } from "@/app/lib/intranet-modules";

const actor = (roles: string[], platformAdmin = false) => ({ userId: "moi", roles, platformAdmin });
const local = { isHome: true, platformAdmin: false };

test("seul un administrateur de l'établissement courant gère les photos des autres", () => {
  assert.equal(canManageOthersAvatars(actor(["professeur"])), false);
  assert.equal(canManageOthersAvatars(actor(["direction_lycee"])), false);
  assert.equal(canManageOthersAvatars(actor(["administratif"])), false);
  assert.equal(canManageAvatarOf(actor(["admin"]), local), true);
  // Admin ailleurs (drapeau global du compte) mais professeur ici : rien.
  assert.equal(canManageAvatarOf({ ...actor(["professeur"]), orgAdmin: true } as never, local), false);
});

test("un compte rattaché ailleurs ou de la plateforme est protégé", () => {
  assert.equal(canManageAvatarOf(actor(["admin"]), { isHome: false, platformAdmin: false }), false);
  assert.equal(canManageAvatarOf(actor(["admin"]), { isHome: true, platformAdmin: true }), false);
  assert.equal(canManageAvatarOf(actor([], true), { isHome: false, platformAdmin: true }), true);
});

test("la photo est recadrée en carré, en JPEG, sans métadonnées", async () => {
  const input = await sharp({ create: { width: 1200, height: 700, channels: 3, background: "#336699" } })
    .withExif({ IFD0: { Copyright: "secret" } })
    .jpeg()
    .toBuffer();
  assert.ok((await sharp(input).metadata()).exif);
  const meta = await sharp(await toAvatarJpeg(input)).metadata();
  assert.deepEqual([meta.format, meta.width, meta.height], ["jpeg", AVATAR_SIDE, AVATAR_SIDE]);
  assert.equal(meta.exif, undefined);
});

test("un fichier qui n'est pas une image est refusé", async () => {
  await assert.rejects(toAvatarJpeg(Buffer.from("<svg onload=alert(1)></svg>")), /Format non reconnu/);
});

test("l'adresse change à chaque nouvelle photo", () => {
  assert.notEqual(avatarUrl("u", new Date(1)), avatarUrl("u", new Date(2)));
});

test("les photos passent le proxy pour le personnel, pas pour les élèves", () => {
  assert.equal(canAccessIntranetPath("/api/avatars/me", ["professeur"], false), true);
  assert.equal(canAccessIntranetPath("/api/avatars", ["maintenance"], false), true);
  assert.equal(canAccessIntranetPath("/api/avatars/me", ["eleve"], false), false);
});
