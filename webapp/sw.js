// Service worker for TfL Tracker
const CACHE_NAME = 'tfl-tracker-cache-v1';
const URLS_TO_CACHE = [
    '/',
    '/index.html',
    '/tfl-tracker.js',
    '/stop_points.json',
    'https://cdn.jsdelivr.net/npm/tailwindcss@2.2.19/dist/tailwind.min.css',
    '/icon.svg',
    '/icons/icon-72x72.png',
    '/icons/icon-96x96.png',
    '/icons/icon-128x128.png',
    '/icons/icon-144x144.png',
    '/icons/icon-152x152.png',
    '/icons/icon-192x192.png',
    '/icons/icon-384x384.png',
    '/icons/icon-512x512.png'
];

// Install event - cache essential files
self.addEventListener('install', event => {
    console.log('Service Worker: Installing...');
    event.waitUntil(
        caches.open(CACHE_NAME)
            .then(cache => {
                console.log('Service Worker: Caching files');
                return cache.addAll(URLS_TO_CACHE);
            })
            .then(() => self.skipWaiting())
    );
});

// Activate event - clean up old caches
self.addEventListener('activate', event => {
    console.log('Service Worker: Activated');
    event.waitUntil(
        caches.keys().then(cacheNames => {
            return Promise.all(
                cacheNames.map(cache => {
                    if (cache !== CACHE_NAME) {
                        console.log('Service Worker: Clearing Old Cache');
                        return caches.delete(cache);
                    }
                })
            );
        })
    );
});

// Fetch event - serve from cache or network with caching
self.addEventListener('fetch', event => {
    // Skip caching for TFL API requests
    if (event.request.url.includes('api.tfl.gov.uk')) {
        return;
    }

    event.respondWith(
        caches.match(event.request)
            .then(response => {
                // Cache hit - return the response from the cached version
                if (response) {
                    return response;
                }

                // Not in cache - fetch from network
                return fetch(event.request)
                    .then(networkResponse => {
                        // Don't cache if not a valid response
                        if (!networkResponse || networkResponse.status !== 200 || networkResponse.type !== 'basic') {
                            return networkResponse;
                        }

                        // Clone the response
                        var responseToCache = networkResponse.clone();

                        // Add to cache
                        caches.open(CACHE_NAME)
                            .then(cache => {
                                cache.put(event.request, responseToCache);
                            });

                        return networkResponse;
                    })
                    .catch(() => {
                        // If both cache and network fail, show a generic fallback
                        if (event.request.url.indexOf('.html') > -1) {
                            return caches.match('/index.html');
                        }

                        // Return nothing if we can't provide anything useful
                        return;
                    });
            })
    );
});

// Handle messages from the client
self.addEventListener('message', event => {
    if (event.data.action === 'skipWaiting') {
        self.skipWaiting();
    }
});