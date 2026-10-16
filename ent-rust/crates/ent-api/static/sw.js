/* ENT labo — cache assets statiques + shell HTML hors ligne (pas de Web Push). */
const CACHE = 'ent-offline-v63';

const PRECACHE = [
  '/',
  '/login',
  '/app',
  '/app/rh',
  '/app/absences',
  '/app/reinscription',
  '/app/mon-edt',
  '/app/edt',
  '/app/etab',
  '/app/notes',
  '/app/conseils',
  '/app/psy',
  '/app/alertes',
  '/app/adultes',
  '/app/eleves',
  '/app/editions',
  '/app/inscriptions',
  '/app/comms',
  '/app/messages',
  '/app/internat',
  '/app/foyers',
  '/app/rh',
  '/app/sorties',
  '/app/facturation',
  '/eleve',
  '/eleve/notes',
  '/eleve/cahier',
  '/eleve/docs',
  '/eleve/messages',
  '/eleve/edt',
  '/eleve/vie-scolaire',
  '/eleve/orientation',
  '/eleve/repas',
  '/eleve/sante',
  '/famille',
  '/famille/enfants',
  '/famille/cahier',
  '/famille/messages',
  '/famille/sorties',
  '/famille/factures',
  '/famille/docs-signer',
  '/famille/reinscription',
  '/famille/orientation',
  '/famille/internat',
  '/famille/sante',
  '/famille/repas',
  '/app/dispenses',
  '/app/cantine',
  '/app/infirmerie',
  '/static/css/app.css',
  '/static/manifest.webmanifest',
  '/static/icon-192.png',
  '/static/icon-512.png',
  '/static/sw.js',
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

function isShellPath(pathname) {
  return (
    pathname === '/'
    || pathname === '/login'
    || pathname === '/app'
    || pathname === '/famille'
    || pathname === '/eleve'
    || pathname.startsWith('/app/')
    || pathname.startsWith('/famille/')
    || pathname.startsWith('/eleve/')
  );
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

  if (!isShellPath(url.pathname)) {
    event.respondWith(networkFirstApi(request));
    return;
  }

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
      if (isShellPath(new URL(request.url).pathname)) {
        await cache.put('/', response.clone());
      }
    }
    return response;
  } catch {
    const cached =
      (await caches.match(request))
      || (await caches.match(new URL(request.url).pathname))
      || (await caches.match('/'));
    if (cached) return cached;
    return new Response('Hors ligne — reconnectez-vous au réseau.', {
      status: 503,
      headers: { 'Content-Type': 'text/plain; charset=utf-8' },
    });
  }
}

async function networkFirstApi(request) {
  try {
    return await fetch(request);
  } catch {
    return new Response(
      JSON.stringify({
        error: 'Hors ligne — API indisponible sans connexion réseau.',
      }),
      {
        status: 503,
        headers: { 'Content-Type': 'application/json; charset=utf-8' },
      }
    );
  }
}
