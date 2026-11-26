import assert from "node:assert/strict";
import { describe, it } from "node:test";
import sharp from "sharp";
import { evaluateFamilyAccess } from "@/app/lib/travels-feed-access";
import {
  applyReaction,
  feedParticipantsForImageRights,
  feedPurgeDecision,
  checkPhotoPublication,
  familyKeyOf,
  FEED_EMAIL_CAP_PER_FAMILY,
  FEED_EMAIL_CAP_PER_TRIP,
  buildFeedUpdateEmail,
  feedPhase,
  feedPostText,
  isFeedPurgeDue,
  pendingDelta,
  planFeedEmail,
  resolveFeedStaffRights,
  stillParticipantInes,
  studentsWithoutImageRight,
  summarizeReactions,
  validateFeedPost,
} from "@/app/lib/travels-feed-logic";
import { storePhotosPost, type PhotoPostDeps } from "@/app/lib/travels-feed-publish";
import { FeedImageError, sanitizeFeedImage, sniffImageType } from "@/app/lib/travels-feed-image";
import { buildInviteToken, generateInviteSecret, hashInviteSecret } from "@/app/lib/travels-v2-registration";

const TRIP_ID = "trip-aaaa-1111";
const ACCESS_ID = "acc-bbbb-2222";

const participant = (ine: string, droitImageOk?: boolean) => ({ ine, nom: `Nom${ine}`, prenom: `Prenom${ine}`, classe: "4A", ...(droitImageOk === undefined ? {} : { droitImageOk }) });

const tripOf = (over: Record<string, unknown> = {}, data: Record<string, unknown> = {}) =>
  ({
    id: TRIP_ID,
    status: "VALIDE",
    ownerId: "u-owner",
    ownerName: "Olivia Owner",
    data: {
      startDate: "2026-07-06",
      endDate: "2026-07-10",
      accompagnateurs: [{ userId: "u-escort", name: "Eric Escort", source: "directory" }],
      participantEleves: [participant("A1", true), participant("B2", false), participant("C3")],
      ...data,
    },
    ...over,
  }) as never;

const user = (id: string, roles: string[] = ["professeur"], email = `${id}@ecole.fr`, fullName = id) =>
  ({ id, fullName, publicMetadata: { role: roles }, primaryEmailAddress: { emailAddress: email } }) as never;

/* ─── Accès des familles ─── */

describe("accès d'une famille au fil", () => {
  const secret = generateInviteSecret();
  const token = buildInviteToken({ invitationId: ACCESS_ID, tripId: TRIP_ID, secret });
  const access = { id: ACCESS_ID, tripId: TRIP_ID, tokenHashes: [hashInviteSecret(secret)], ines: ["A1"], familyKey: "A1" };
  const now = new Date("2026-07-08T10:00:00Z");

  it("famille d'un élève participant : oui", () => {
    const d = evaluateFamilyAccess({ token, access, trip: tripOf(), feedOpen: true, now });
    assert.equal(d.ok, true);
    if (d.ok) assert.deepEqual(d.ines, ["A1"]);
  });

  it("famille d'un élève DÉSISTÉ ou retiré de la liste officielle : non", () => {
    const d = evaluateFamilyAccess({ token, access, trip: tripOf({}, { participantEleves: [participant("B2", true)] }), feedOpen: true, now });
    assert.equal(d.ok, false);
  });

  it("pré-liste seule (aucune liste officielle) : non", () => {
    const d = evaluateFamilyAccess({ token, access, trip: tripOf({}, { participantEleves: [], preListeEleves: [participant("A1", true)] }), feedOpen: true, now });
    assert.equal(d.ok, false);
  });

  it("jeton invalide, mal formé ou secret faux : même refus uniforme", () => {
    const wrong = buildInviteToken({ invitationId: ACCESS_ID, tripId: TRIP_ID, secret: generateInviteSecret() });
    for (const t of [wrong, "n'importe quoi", "", null, undefined, `${ACCESS_ID}.${TRIP_ID}.court`]) {
      assert.deepEqual(evaluateFamilyAccess({ token: t as never, access, trip: tripOf(), feedOpen: true, now }), { ok: false });
    }
  });

  it("jeton d'un AUTRE voyage : non", () => {
    const other = { ...access, tripId: "trip-other-9999" };
    assert.equal(evaluateFamilyAccess({ token, access: other, trip: tripOf(), feedOpen: true, now }).ok, false);
    const otherTrip = tripOf({ id: "trip-other-9999" });
    assert.equal(evaluateFamilyAccess({ token, access, trip: otherTrip, feedOpen: true, now }).ok, false);
  });

  it("fil non ouvert, accès ou voyage absents : non", () => {
    assert.equal(evaluateFamilyAccess({ token, access, trip: tripOf(), feedOpen: false, now }).ok, false);
    assert.equal(evaluateFamilyAccess({ token, access: null, trip: tripOf(), feedOpen: true, now }).ok, false);
    assert.equal(evaluateFamilyAccess({ token, access, trip: null, feedOpen: true, now }).ok, false);
  });

  it("voyage annulé : non ; purge échue : non ; après le retour : lecture seule", () => {
    assert.equal(evaluateFamilyAccess({ token, access, trip: tripOf({ status: "ANNULE" }), feedOpen: true, now }).ok, false);
    assert.equal(evaluateFamilyAccess({ token, access, trip: tripOf(), feedOpen: true, now: new Date("2026-07-17T08:00:00Z") }).ok, false);
    const ro = evaluateFamilyAccess({ token, access, trip: tripOf(), feedOpen: true, now: new Date("2026-07-12T08:00:00Z") });
    assert.equal(ro.ok && ro.phase, "readonly");
  });

  it("élève retiré de la famille mais autre enfant participant : l'accès reste, limité aux participants", () => {
    const two = { ...access, ines: ["A1", "Z9"] };
    const d = evaluateFamilyAccess({ token, access: two, trip: tripOf(), feedOpen: true, now });
    assert.equal(d.ok && d.ines.join(), "A1");
  });

  it("clé de famille : premier INE trié, partagée par les deux responsables", () => {
    assert.equal(familyKeyOf(["b2", "A1"]), "A1");
    assert.deepEqual(stillParticipantInes(["a1", "x"], [{ ine: "A1" }]), ["a1"]);
  });
});

/* ─── Droits des profs ─── */

describe("droits de publication et de modération", () => {
  it("organisateur et accompagnateur du voyage : publient", () => {
    assert.equal(resolveFeedStaffRights(tripOf(), user("u-owner"), false).canPublish, true);
    assert.equal(resolveFeedStaffRights(tripOf(), user("u-escort"), false).canPublish, true);
  });
  it("accompagnateur d'un AUTRE voyage : ni publier ni modérer", () => {
    const t = tripOf({}, { accompagnateurs: [{ userId: "u-other", name: "Z", source: "directory" }] });
    const r = resolveFeedStaffRights(t, user("u-escort"), false);
    assert.equal(r.canPublish, false);
    assert.equal(r.canModerate, false);
  });
  it("homonyme : non", () => {
    assert.equal(resolveFeedStaffRights(tripOf(), user("u-zz", ["professeur"], "zz@x.fr", "Olivia Owner"), false).canPublish, false);
  });
  it("modération : organisateur, direction, admin ; pas l'accompagnateur", () => {
    assert.equal(resolveFeedStaffRights(tripOf(), user("u-owner"), false).canModerate, true);
    assert.equal(resolveFeedStaffRights(tripOf(), user("u-dir"), true).canModerate, true);
    assert.equal(resolveFeedStaffRights(tripOf(), user("u-adm", ["admin"]), false).canModerate, true);
    assert.equal(resolveFeedStaffRights(tripOf(), user("u-escort"), false).canModerate, false);
  });
  it("direction seule : modère mais ne publie pas", () => {
    const r = resolveFeedStaffRights(tripOf(), user("u-dir"), true);
    assert.equal(r.canPublish, false);
    assert.equal(r.canModerate, true);
  });
  it("utilisateur absent : rien", () => {
    assert.equal(resolveFeedStaffRights(tripOf(), null, true).canModerate, false);
  });
});

/* ─── Publications ─── */

describe("publications", () => {
  it("raccourcis sans texte", () => {
    for (const kind of ["arrives", "ok", "departure_back", "at_school"]) {
      const v = validateFeedPost({ kind });
      assert.equal(v.ok, true);
    }
    assert.equal(feedPostText({ kind: "arrives", body: null, delayMinutes: null }), "Bien arrivés !");
  });
  it("petit retard : durée obligatoire et bornée", () => {
    assert.equal(validateFeedPost({ kind: "delay" }).ok, false);
    assert.equal(validateFeedPost({ kind: "delay", delayMinutes: 2 }).ok, false);
    const ok = validateFeedPost({ kind: "delay", delayMinutes: 30 });
    assert.equal(ok.ok, true);
    assert.match(feedPostText({ kind: "delay", body: null, delayMinutes: 30 }), /30 min/);
  });
  it("message : 500 caractères maximum, jamais vide", () => {
    assert.equal(validateFeedPost({ kind: "message", body: "   " }).ok, false);
    assert.equal(validateFeedPost({ kind: "message", body: "a".repeat(500) }).ok, true);
    assert.equal(validateFeedPost({ kind: "message", body: "a".repeat(501) }).ok, false);
  });
  it("type inconnu ou « photos » par ce circuit : refusé", () => {
    assert.equal(validateFeedPost({ kind: "photos" }).ok, false);
    assert.equal(validateFeedPost({ kind: "nimporte" }).ok, false);
  });
});

/* ─── Droit à l'image ─── */

describe("droit à l'image des photos", () => {
  const list = studentsWithoutImageRight([participant("A1", true), participant("B2", false), participant("C3")] as never);

  it("liste les élèves refusés ET non renseignés, pas ceux qui ont dit oui", () => {
    assert.deepEqual(list.map((s) => [s.ine, s.status]), [
      ["B2", "refuse"],
      ["C3", "non_renseigne"],
    ]);
  });
  it("case obligatoire dès que la liste n'est pas vide", () => {
    assert.equal(checkPhotoPublication({ listed: list, acknowledged: false, photoCount: 2 }).ok, false);
    assert.equal(checkPhotoPublication({ listed: list, acknowledged: true, photoCount: 2 }).ok, true);
  });
  it("tous les élèves ont dit oui : pas de case requise", () => {
    assert.equal(checkPhotoPublication({ listed: [], acknowledged: false, photoCount: 1 }).ok, true);
  });
  it("0 photo ou plus de 10 : refusé", () => {
    assert.equal(checkPhotoPublication({ listed: [], acknowledged: true, photoCount: 0 }).ok, false);
    assert.equal(checkPhotoPublication({ listed: [], acknowledged: true, photoCount: 11 }).ok, false);
  });
});

/* ─── Réactions ─── */

describe("réactions des familles", () => {
  it("une par type, retirable, jamais deux fois", () => {
    let mine: string[] = [];
    mine = applyReaction(mine, "👍", true);
    mine = applyReaction(mine, "👍", true);
    assert.deepEqual(mine, ["👍"]);
    mine = applyReaction(mine, "❤️", true);
    assert.deepEqual(mine, ["👍", "❤️"]);
    mine = applyReaction(mine, "👍", false);
    assert.deepEqual(mine, ["❤️"]);
  });
  it("emoji hors liste ignoré (pas de texte libre déguisé)", () => {
    assert.deepEqual(applyReaction([], "mon commentaire", true), []);
  });
  it("compteurs sans noms, dédoublonnés par famille", () => {
    const rows = [
      { familyKey: "A1", emoji: "👍" },
      { familyKey: "A1", emoji: "👍" },
      { familyKey: "B2", emoji: "👍" },
      { familyKey: "B2", emoji: "😄" },
    ];
    const s = summarizeReactions(rows, "A1");
    assert.deepEqual(s.counts, { "👍": 2, "😄": 1 });
    assert.deepEqual(s.mine, ["👍"]);
    assert.equal(JSON.stringify(s).includes("B2"), false);
  });
});

/* ─── E-mails regroupés ─── */

describe("e-mails regroupés", () => {
  const now = new Date("2026-07-08T12:00:00Z");
  const base = { lastNotifiedAt: null, pendingMessages: 1, pendingPhotos: 0, notifyCount: 0 };

  it("premier e-mail : part tout de suite", () => {
    assert.deepEqual(planFeedEmail({ state: base, now, tripEmailsSent: 0 }), { send: true });
  });
  it("au plus un par famille et par heure", () => {
    const recent = new Date(now.getTime() - 59 * 60_000);
    assert.deepEqual(planFeedEmail({ state: { ...base, lastNotifiedAt: recent }, now, tripEmailsSent: 0 }), { send: false, reason: "too_soon" });
    const old = new Date(now.getTime() - 61 * 60_000);
    assert.deepEqual(planFeedEmail({ state: { ...base, lastNotifiedAt: old }, now, tripEmailsSent: 0 }), { send: true });
  });
  it("rien en attente : rien", () => {
    assert.deepEqual(planFeedEmail({ state: { ...base, pendingMessages: 0 }, now, tripEmailsSent: 0 }), { send: false, reason: "nothing" });
  });
  it("plafonds par famille et par voyage", () => {
    assert.deepEqual(planFeedEmail({ state: { ...base, notifyCount: FEED_EMAIL_CAP_PER_FAMILY }, now, tripEmailsSent: 0 }), { send: false, reason: "family_cap" });
    assert.deepEqual(planFeedEmail({ state: base, now, tripEmailsSent: FEED_EMAIL_CAP_PER_TRIP }), { send: false, reason: "trip_cap" });
  });
  const D = (h: number, m = 0) => new Date(Date.UTC(2026, 6, 8, h, m));
  const item = (over: Record<string, unknown>) => ({ kind: "ok", body: null, delayMinutes: null, authorName: "Eric Escort", createdAt: D(12), photoCount: 0, ...over }) as never;
  const mailOf = (items: unknown[]) =>
    buildFeedUpdateEmail({ destination: "Biarritz", tripTitle: "Sortie surf", school: "Collège X", link: "https://x.fr/voyages/famille/fil/T", items: items as never })!;
  it("un seul statut : objet explicite, texte, heure et prénom de l'auteur", () => {
    const m = mailOf([item({ kind: "arrives" })]);
    assert.equal(m.subject, "Sortie à Biarritz : nous sommes bien arrivés");
    assert.match(m.text, /Bien arrivés/);
    assert.match(m.text, /Eric/);
    assert.equal(m.text.includes("Escort"), false); // prénom seulement
    assert.match(m.text, /14:00/); // heure de Paris (UTC+2)
    assert.ok(m.text.includes("https://x.fr/voyages/famille/fil/T"));
  });
  it("petit retard : durée dans l'objet", () => {
    assert.equal(mailOf([item({ kind: "delay", delayMinutes: 30 })]).subject, "Sortie à Biarritz : petit retard (~30 min)");
  });
  it("plusieurs publications : « N nouvelles informations », liste chronologique", () => {
    const m = mailOf([item({ kind: "message", body: "Second", createdAt: D(14) }), item({ kind: "program", body: "Premier", createdAt: D(9) }), item({ kind: "ok", createdAt: D(11) })]);
    assert.equal(m.subject, "Sortie à Biarritz : 3 nouvelles informations");
    assert.ok(m.text.indexOf("Premier") < m.text.indexOf("Tout va bien"));
    assert.ok(m.text.indexOf("Tout va bien") < m.text.indexOf("Second"));
  });
  it("photos : jamais dans l'e-mail, seulement le nombre et le lien", () => {
    const m = mailOf([item({ kind: "photos", body: "légende secrète", photoCount: 3 })]);
    assert.equal(m.subject, "Sortie à Biarritz : 3 nouvelles photos");
    assert.match(m.text, /3 nouvelles photos — à voir sur votre page/);
    assert.equal(m.text.includes("légende secrète"), false);
    assert.equal(m.html.includes("<img"), false);
    const mixed = mailOf([item({ kind: "ok" }), item({ kind: "photos", photoCount: 2 })]);
    assert.match(mixed.text, /2 nouvelles photos/);
    assert.match(mixed.subject, /2 nouvelles informations/);
  });
  it("texte du prof échappé en HTML, repris tel quel en texte brut", () => {
    const m = mailOf([item({ kind: "message", body: "<script>alert(1)</script> & \"ok\"" })]);
    assert.equal(m.html.includes("<script>"), false);
    assert.match(m.html, /&lt;script&gt;/);
    assert.ok(m.text.includes("<script>alert(1)</script>"));
  });
  it("aucune publication : pas d'e-mail ; objet sans retour à la ligne", () => {
    assert.equal(buildFeedUpdateEmail({ destination: "", tripTitle: "T", school: "S", link: "l", items: [] }), null);
    const m = buildFeedUpdateEmail({ destination: "Bia" + String.fromCharCode(13, 10) + "rritz", tripTitle: "T", school: "S", link: "l", items: [item({})] })!;
    assert.equal(/[\r\n]/.test(m.subject), false);
  });
  it("compteurs : photos d'un côté, le reste de l'autre", () => {
    assert.deepEqual(pendingDelta("photos"), { messages: 0, photos: 1 });
    assert.deepEqual(pendingDelta("ok"), { messages: 1, photos: 0 });
  });
});

/* ─── Durée de vie ─── */

describe("lecture seule et purge", () => {
  const ret = "2026-07-10";
  it("en direct jusqu'au jour du retour inclus", () => {
    assert.equal(feedPhase({ status: "VALIDE", returnIso: ret, todayIso: "2026-07-10" }), "live");
  });
  it("lecture seule après la date de retour", () => {
    assert.equal(feedPhase({ status: "VALIDE", returnIso: ret, todayIso: "2026-07-11" }), "readonly");
    assert.equal(feedPhase({ status: "VALIDE", returnIso: ret, todayIso: "2026-07-16" }), "readonly");
  });
  it("purge à retour + 7 jours", () => {
    assert.equal(feedPhase({ status: "VALIDE", returnIso: ret, todayIso: "2026-07-17" }), "purge_due");
    assert.equal(isFeedPurgeDue({ status: "VALIDE", returnIso: ret, todayIso: "2026-07-16" }), false);
    assert.equal(isFeedPurgeDue({ status: "VALIDE", returnIso: ret, todayIso: "2026-07-17" }), true);
  });
  it("voyage annulé : purgé ; sans date de retour : jamais ouvrable ni purgé à tort", () => {
    assert.equal(isFeedPurgeDue({ status: "ANNULE", returnIso: ret, todayIso: "2026-07-01" }), true);
    assert.equal(feedPhase({ status: "VALIDE", returnIso: null, todayIso: "2026-07-01" }), "unavailable");
    assert.equal(isFeedPurgeDue({ status: "VALIDE", returnIso: null, todayIso: "2030-01-01" }), false);
  });
});

/* ─── Photos : octets contrôlés, EXIF/GPS retirés ─── */

describe("nettoyage des photos", () => {
  const gpsExif = {
    IFD0: { Make: "CameraSecrete", Copyright: "Famille Dupont" },
    IFD3: { GPSLatitudeRef: "N", GPSLatitude: "48/1 51/1 24/1", GPSLongitudeRef: "E", GPSLongitude: "2/1 21/1 7/1" },
  };

  it("retire l'EXIF (dont le GPS) et redimensionne à 1600 px", async () => {
    const input = await sharp({ create: { width: 3200, height: 2000, channels: 3, background: { r: 200, g: 30, b: 30 } } })
      .jpeg()
      .withExif(gpsExif)
      .toBuffer();
    // Le jeu d'essai contient bien des métadonnées sensibles.
    assert.ok(input.includes(Buffer.from("Exif")));
    assert.ok(input.includes(Buffer.from("CameraSecrete")));
    assert.ok((await sharp(input).metadata()).exif);

    const out = await sanitizeFeedImage(input);
    assert.equal(out.contentType, "image/jpeg");
    assert.equal(Math.max(out.width, out.height), 1600);
    assert.equal(sniffImageType(out.bytes), "jpeg");
    assert.equal(out.bytes.includes(Buffer.from("Exif")), false);
    assert.equal(out.bytes.includes(Buffer.from("CameraSecrete")), false);
    assert.equal(out.bytes.includes(Buffer.from("Dupont")), false);
    const meta = await sharp(out.bytes).metadata();
    assert.equal(meta.exif, undefined);
    assert.equal(meta.xmp, undefined);
    assert.equal(meta.iptc, undefined);
  });

  it("accepte PNG et WebP, sort toujours en JPEG", async () => {
    const png = await sharp({ create: { width: 100, height: 80, channels: 4, background: { r: 0, g: 0, b: 255, alpha: 0.5 } } }).png().toBuffer();
    const webp = await sharp({ create: { width: 100, height: 80, channels: 3, background: { r: 0, g: 255, b: 0 } } }).webp().toBuffer();
    assert.equal(sniffImageType(png), "png");
    assert.equal(sniffImageType(webp), "webp");
    for (const b of [png, webp]) assert.equal(sniffImageType((await sanitizeFeedImage(b)).bytes), "jpeg");
  });

  it("contrôle les OCTETS : un fichier non image (même nommé .jpg) est refusé", async () => {
    await assert.rejects(() => sanitizeFeedImage(Buffer.from("<html><script>alert(1)</script></html>padding padding")), FeedImageError);
    await assert.rejects(() => sanitizeFeedImage(Buffer.from("MZ" + "x".repeat(200))), FeedImageError);
  });

  it("JPEG tronqué / corrompu : refusé proprement", async () => {
    const good = await sharp({ create: { width: 400, height: 300, channels: 3, background: { r: 1, g: 2, b: 3 } } }).jpeg().toBuffer();
    const broken = Buffer.concat([good.subarray(0, 40), Buffer.alloc(200, 7)]);
    await assert.rejects(() => sanitizeFeedImage(broken), FeedImageError);
  });

  it("HEIC illisible côté serveur : message clair (le navigateur le convertit en JPEG)", async () => {
    const fakeHeic = Buffer.concat([Buffer.from([0, 0, 0, 24]), Buffer.from("ftypheic"), Buffer.alloc(60)]);
    assert.equal(sniffImageType(fakeHeic), "heic");
    await assert.rejects(() => sanitizeFeedImage(fakeHeic), /HEIC/);
  });
});

describe("droit à l'image : valeur brute de la pré-liste (fil uniquement)", () => {
  it("« non renseigné » dans la pré-liste n'est PAS transformé en « oui » : listé, case exigée", () => {
    const t = tripOf({}, {
      participantEleves: [participant("A1", true), participant("B2", true), participant("C3", true)], // valeurs forcées à true par la lecture SQL
      preListeEleves: [participant("A1", true), participant("B2"), participant("C3", false)],
    });
    const listed = studentsWithoutImageRight(feedParticipantsForImageRights(t));
    assert.deepEqual(listed.map((s) => [s.ine, s.status]), [["B2", "non_renseigne"], ["C3", "refuse"]]);
    assert.equal(checkPhotoPublication({ listed, acknowledged: false, photoCount: 1 }).ok, false);
  });
  it("élève absent de la pré-liste : valeur de la liste officielle", () => {
    const t = tripOf({}, { participantEleves: [participant("A1", false)], preListeEleves: [] });
    assert.deepEqual(studentsWithoutImageRight(feedParticipantsForImageRights(t)).map((s) => s.status), ["refuse"]);
  });
});

describe("purge prudente du fil", () => {
  const opened = new Date("2026-06-01T10:00:00Z");
  const base = { openedAt: opened, todayIso: "2026-07-01" };
  it("voyage illisible (base/cache indisponible) : JAMAIS de purge sur ce seul motif", () => {
    assert.equal(feedPurgeDecision({ ...base, tripState: "unknown", returnIso: null, todayIso: "2026-06-05" }), false);
    assert.equal(feedPurgeDecision({ ...base, tripState: "unknown", returnIso: "2026-07-20" }), false);
  });
  it("voyage illisible mais date de retour mémorisée échue (J+7) : purge", () => {
    assert.equal(feedPurgeDecision({ ...base, tripState: "unknown", returnIso: "2026-06-20" }), true);
  });
  it("absence CONFIRMÉE par lecture directe : purge ; annulé : purge", () => {
    assert.equal(feedPurgeDecision({ ...base, tripState: "absent" }), true);
    assert.equal(feedPurgeDecision({ ...base, tripState: "found", status: "ANNULE", returnIso: "2026-12-01" }), true);
  });
  it("sans date de retour : ouverture + 30 jours", () => {
    assert.equal(feedPurgeDecision({ openedAt: opened, tripState: "found", status: "VALIDE", returnIso: null, todayIso: "2026-06-30" }), false);
    assert.equal(feedPurgeDecision({ openedAt: opened, tripState: "found", status: "VALIDE", returnIso: null, todayIso: "2026-07-01" }), true);
  });
});

describe("publication de photos tout ou rien", () => {
  const cleaned = [1, 2, 3].map(() => ({ bytes: Buffer.from("x"), width: 10, height: 10 }));
  const mk = (failPhotos: boolean, failPut = false) => {
    const log = { puts: [] as string[], deletedObjects: [] as string[], posts: new Set<string>(), photos: 0 };
    let n = 0;
    const deps: PhotoPostDeps = {
      newId: () => `id${++n}`,
      putObject: async (path) => {
        if (failPut && log.puts.length === 1) throw new Error("S3 en panne");
        log.puts.push(path);
        return path;
      },
      insertPost: async () => {
        log.posts.add("post1");
        return { id: "post1" };
      },
      insertPhotos: async () => {
        if (failPhotos) throw new Error("base en panne");
        log.photos = 3;
      },
      deletePost: async (id) => void log.posts.delete(id),
      deleteObject: async (k) => void log.deletedObjects.push(k),
    };
    return { deps, log };
  };
  it("succès : une publication, trois photos", async () => {
    const { deps, log } = mk(false);
    const r = await storePhotosPost(deps, { pathPrefix: "p", cleaned });
    assert.equal(r.postId, "post1");
    assert.equal(log.posts.size, 1);
    assert.equal(log.photos, 3);
  });
  it("échec d'insertion des photos : la publication est supprimée (jamais de post vide) et les objets retirés", async () => {
    const { deps, log } = mk(true);
    await assert.rejects(() => storePhotosPost(deps, { pathPrefix: "p", cleaned }), /base en panne/);
    assert.equal(log.posts.size, 0);
    assert.equal(log.deletedObjects.length, 3);
  });
  it("échec de dépôt d'un objet : aucune publication créée, objets déjà déposés retirés", async () => {
    const { deps, log } = mk(false, true);
    await assert.rejects(() => storePhotosPost(deps, { pathPrefix: "p", cleaned }), /S3/);
    assert.equal(log.posts.size, 0);
    assert.equal(log.deletedObjects.length, 1);
  });
});
