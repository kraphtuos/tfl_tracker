// Service worker for TfL Tracker
// Bump the version when changing what is cached or how.
const CACHE_NAME = 'tfl-tracker-v4';

// Relative to this file so the app also works from a sub-path (e.g. GitHub Pages)
const PRECACHE_URLS = [
    './',
    './index.html',
    './manifest.json',
    './stop_points.json',
    './icons/favicon.svg',
    './icons/favicon-96x96.png',
    './icons/apple-touch-icon-180x180.png',
    './icons/icon-192x192.png',
    './icons/icon-512x512.png',
];

// Trunk puts a content hash in built asset names (e.g. tfl_tracker-1a2b3c4d_bg.wasm),
// so a cached copy never goes stale. Captures the name without the hash.
const HASHED_ASSET = /^(.*)-[0-9a-f]{8,}((?:_bg)?\.(?:js|wasm|css))$/;

self.addEventListener('install', event => {
    event.waitUntil(
        caches.open(CACHE_NAME)
            // Bypass the HTTP cache so a new version never precaches stale copies
            .then(cache => cache.addAll(PRECACHE_URLS.map(url => new Request(url, { cache: 'reload' }))))
            .then(() => self.skipWaiting())
    );
});

self.addEventListener('activate', event => {
    event.waitUntil(
        caches.keys()
            .then(names => Promise.all(
                names.filter(name => name !== CACHE_NAME).map(name => caches.delete(name))
            ))
            .then(() => self.clients.claim())
    );
});

self.addEventListener('fetch', event => {
    const request = event.request;
    if (request.method !== 'GET') {
        return;
    }
    // Live data (TfL API) and other origins always go to the network
    const url = new URL(request.url);
    if (url.origin !== self.location.origin) {
        return;
    }

    if (HASHED_ASSET.test(url.pathname)) {
        event.respondWith(cacheFirst(request));
    } else if (url.pathname.endsWith('/stop_points.json')) {
        // Station data rarely changes and the app can't start without it; don't wait on the network
        event.respondWith(staleWhileRevalidate(request, event));
    } else {
        // The page, manifest and icons: fresh when online. iOS reads the icon only once,
        // when the app is added to the Home Screen, so it must not get a stale copy.
        event.respondWith(networkFirst(request));
    }
});

async function staleWhileRevalidate(request, event) {
    const cache = await caches.open(CACHE_NAME);
    const cached = await cache.match(request, { ignoreSearch: true });
    // Revalidate with the server rather than reuse the HTTP cache (GitHub Pages allows 10 minutes)
    const network = fetch(request, { cache: 'no-cache' }).then(async response => {
        if (response.ok) {
            await cache.put(request, response.clone());
        }
        return response;
    });
    if (cached) {
        event.waitUntil(network.catch(() => {}));
        return cached;
    }
    return network;
}

// On a weak signal (common underground) fall back to the cached copy after this long
const NETWORK_TIMEOUT_MS = 4000;

// Network first so the page stays fresh after a deploy; cache is the fallback
// when offline or when the network is too slow
async function networkFirst(request) {
    const cache = await caches.open(CACHE_NAME);
    // Revalidate with the server rather than reuse the HTTP cache (GitHub Pages allows 10 minutes)
    const network = fetch(request, { cache: 'no-cache' }).then(async response => {
        if (response.ok) {
            await cache.put(request, response.clone());
        }
        return response;
    });
    // Still let a slow request finish and refresh the cache in the background
    network.catch(() => {});

    try {
        return await withTimeout(network, NETWORK_TIMEOUT_MS);
    } catch (error) {
        const cached = await cachedCopy(cache, request);
        // Nothing cached: keep waiting for the network (or report its failure)
        return cached || network;
    }
}

function withTimeout(promise, ms) {
    return new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error('Network timeout')), ms);
        promise.then(
            value => { clearTimeout(timer); resolve(value); },
            error => { clearTimeout(timer); reject(error); }
        );
    });
}

async function cachedCopy(cache, request) {
    const cached = await cache.match(request, { ignoreSearch: true });
    if (cached || request.mode !== 'navigate') {
        return cached;
    }
    return cache.match('./index.html');
}

async function cacheFirst(request) {
    const cache = await caches.open(CACHE_NAME);
    const cached = await cache.match(request);
    if (cached) {
        return cached;
    }
    const response = await fetch(request);
    if (response.ok) {
        await removeOldVersions(cache, request.url);
        await cache.put(request, response.clone());
    }
    return response;
}

// Drop previous builds of a hashed asset so the cache doesn't grow with every deploy
async function removeOldVersions(cache, newUrl) {
    const unhashed = url => {
        const match = new URL(url).pathname.match(HASHED_ASSET);
        return match && match[1] + match[2];
    };
    const name = unhashed(newUrl);
    const keys = await cache.keys();
    await Promise.all(
        keys
            .filter(key => key.url !== newUrl && unhashed(key.url) === name)
            .map(key => cache.delete(key))
    );
}
