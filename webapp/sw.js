// Service worker for TfL Tracker
// Bump the version when changing what is cached or how.
const CACHE_NAME = 'tfl-tracker-v2';

// Relative to this file so the app also works from a sub-path (e.g. GitHub Pages)
const PRECACHE_URLS = [
    './',
    './index.html',
    './manifest.json',
    './stop_points.json',
    './icons/favicon.svg',
    './icons/favicon-96x96.png',
    './icons/apple-touch-icon.png',
    './icons/icon-192x192.png',
    './icons/icon-512x512.png',
];

// Trunk puts a content hash in built asset names (e.g. tfl_tracker-1a2b3c4d_bg.wasm),
// so a cached copy never goes stale. Captures the name without the hash.
const HASHED_ASSET = /^(.*)-[0-9a-f]{8,}((?:_bg)?\.(?:js|wasm|css))$/;

self.addEventListener('install', event => {
    event.waitUntil(
        caches.open(CACHE_NAME)
            .then(cache => cache.addAll(PRECACHE_URLS))
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

    event.respondWith(
        HASHED_ASSET.test(url.pathname) ? cacheFirst(request) : networkFirst(request)
    );
});

// Network first so pages and station data stay fresh; cache is the offline fallback
async function networkFirst(request) {
    const cache = await caches.open(CACHE_NAME);
    try {
        const response = await fetch(request);
        if (response.ok) {
            await cache.put(request, response.clone());
        }
        return response;
    } catch (error) {
        const cached = await cache.match(request, { ignoreSearch: true });
        if (cached) {
            return cached;
        }
        if (request.mode === 'navigate') {
            const page = await cache.match('./index.html');
            if (page) {
                return page;
            }
        }
        throw error;
    }
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
