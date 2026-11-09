import { test } from "node:test";
import assert from "node:assert/strict";
import { isAllowedPushEndpoint } from "./endpoint";

test("services de push connus acceptés", () => {
  assert.equal(isAllowedPushEndpoint("https://fcm.googleapis.com/fcm/send/abc"), true);
  assert.equal(isAllowedPushEndpoint("https://updates.push.services.mozilla.com/wpush/v2/x"), true);
  assert.equal(isAllowedPushEndpoint("https://wns2-par02p.notify.windows.com/w/?token=x"), true);
  assert.equal(isAllowedPushEndpoint("https://web.push.apple.com/QGx"), true);
});

test("hôtes arbitraires, internes ou piégés refusés", () => {
  assert.equal(isAllowedPushEndpoint("https://10.0.0.5/x"), false);
  assert.equal(isAllowedPushEndpoint("https://localhost/x"), false);
  assert.equal(isAllowedPushEndpoint("http://fcm.googleapis.com/x"), false);
  assert.equal(isAllowedPushEndpoint("https://fcm.googleapis.com:8443/x"), false);
  assert.equal(isAllowedPushEndpoint("https://user:pw@fcm.googleapis.com/x"), false);
  assert.equal(isAllowedPushEndpoint("https://fcm.googleapis.com.evil.fr/x"), false);
  assert.equal(isAllowedPushEndpoint("https://evilnotify.windows.com/x"), false);
  assert.equal(isAllowedPushEndpoint("https://.notify.windows.com/x"), false);
  assert.equal(isAllowedPushEndpoint("pas une url"), false);
});
