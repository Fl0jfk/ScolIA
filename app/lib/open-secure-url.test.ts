import assert from "node:assert/strict";
import test from "node:test";
import { ongletPreouvertNonPilotable } from "./open-secure-url";

const UA = {
  iphoneSafari:
    "Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Mobile/15E148 Safari/604.1",
  iphoneChrome:
    "Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/131.0.6778.73 Mobile/15E148 Safari/604.1",
  ipadOs:
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Safari/605.1.15",
  macSafari:
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Safari/605.1.15",
  macChrome:
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
  windowsChrome:
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
  windowsEdge:
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0",
  windowsFirefox: "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0",
  androidChrome:
    "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36",
};

test("iPhone (Safari comme Chrome) : ouverture dans la page", () => {
  assert.equal(ongletPreouvertNonPilotable({ userAgent: UA.iphoneSafari, maxTouchPoints: 5, autonome: false }), true);
  assert.equal(ongletPreouvertNonPilotable({ userAgent: UA.iphoneChrome, maxTouchPoints: 5, autonome: false }), true);
});

test("iPadOS (se présente comme un Mac, mais tactile) : ouverture dans la page", () => {
  assert.equal(ongletPreouvertNonPilotable({ userAgent: UA.ipadOs, maxTouchPoints: 5, autonome: false }), true);
  assert.equal(ongletPreouvertNonPilotable({ userAgent: UA.ipadOs, maxTouchPoints: 1, autonome: false }), true);
});

test("application installée (écran d'accueil), Android compris : ouverture dans la page", () => {
  assert.equal(ongletPreouvertNonPilotable({ userAgent: UA.androidChrome, maxTouchPoints: 5, autonome: true }), true);
  assert.equal(ongletPreouvertNonPilotable({ userAgent: UA.windowsChrome, maxTouchPoints: 0, autonome: true }), true);
});

test("ordinateurs et navigateurs Android : nouvel onglet conservé", () => {
  for (const ua of [UA.macSafari, UA.macChrome, UA.windowsChrome, UA.windowsEdge, UA.windowsFirefox]) {
    assert.equal(ongletPreouvertNonPilotable({ userAgent: ua, maxTouchPoints: 0, autonome: false }), false, ua);
  }
  assert.equal(ongletPreouvertNonPilotable({ userAgent: UA.androidChrome, maxTouchPoints: 5, autonome: false }), false);
});

test("agent utilisateur vide : nouvel onglet conservé", () => {
  assert.equal(ongletPreouvertNonPilotable({ userAgent: "", maxTouchPoints: 0, autonome: false }), false);
});
