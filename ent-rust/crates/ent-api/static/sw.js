/* ENT labo — cache assets statiques + shell HTML hors ligne (pas de Web Push). */
const CACHE = 'ent-offline-v1';

const PRECACHE = [
  '/',
  '/static/css/app.css',
  '/static/manifest.webmanifest',
  '/static/icon-192.png',
  '/static/icon-512.png',
];

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches
      .open(CACHE)
      .then((cache) => cache.addAll(PRECACHE))
      .then(() => self.skipWaiting())
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(keys.filter((name) => name !== CACHE).map((name) => caches.delete(name)))
      )
      .then(() => self.clients.claim())
  );
});

function sameOrigin(url) {
  return url.origin === self.location.origin;
}

function isStaticPath(pathname) {
  return pathname.startsWith('/static/');
}

self.addEventListener('fetch', (event) => {
  const { request } = event;
  const url = new URL(request.url);
  if (!sameOrigin(url)) return;

  if (isStaticPath(url.pathname)) {
    event.respondWith(cacheFirst(request));
    return;
  }

  if (request.mode === 'navigate') {
    event.respondWith(networkFirstShell(request));
    return;
  }

  /* API / JSON / mutations : réseau uniquement (pas de cache stale). */
  event.respondWith(fetch(request));
});

async function cacheFirst(request) {
  const cached = await caches.match(request);
  if (cached) return cached;
  try {
    const response = await fetch(request);
    if (response.ok) {
      const cache = await caches.open(CACHE);
      await cache.put(request, response.clone());
    }
    return response;
  } catch {
    return cached || Response.error();
  }
}

async function networkFirstShell(request) {
  try {
    const response = await fetch(request);
    if (response.ok) {
      const cache = await caches.open(CACHE);
      await cache.put(request, response.clone());
    }
    return response;
  } catch {
    const cached = (await caches.match(request)) || (await caches.match('/'));
    if (cached) return cached;
    return new Response('Hors ligne — reconnectez-vous au réseau.', {
      status: 503,
      headers: { 'Content-Type': 'text/plain; charset=utf-8' },
    });
  }
}
