import { browser } from '$app/environment';

const SCROLL_PREFIX = 'duskytunes:v20:scroll:';
const LAST_ROUTE = 'duskytunes:v20:last-route';
const ROUTE_TRAIL = 'duskytunes:route-trail:v1';
const MAX_TRAIL = 64;

function safeGet(key: string): string | null {
	if (!browser) return null;
	try { return sessionStorage.getItem(key); } catch { return null; }
}

function safeSet(key: string, value: string): void {
	if (!browser) return;
	try { sessionStorage.setItem(key, value); } catch {}
}

function safeJson<T>(key: string, fallback: T): T {
	try {
		const raw = safeGet(key);
		return raw ? (JSON.parse(raw) as T) : fallback;
	} catch {
		return fallback;
	}
}

export function routeKey(url: URL | string): string {
	if (typeof url === 'string') return url || '/';
	return `${url.pathname}${url.search}` || '/';
}

function readTrail(): string[] {
	const trail = safeJson<string[]>(ROUTE_TRAIL, ['/']).filter((x) => typeof x === 'string' && x.startsWith('/'));
	if (!trail.length) return ['/'];
	if (trail[0] !== '/') trail.unshift('/');
	return trail.slice(-MAX_TRAIL);
}

function writeTrail(trail: string[]): void {
	const clean = trail.length ? trail.slice(-MAX_TRAIL) : ['/'];
	if (clean[0] !== '/') clean.unshift('/');
	safeSet(ROUTE_TRAIL, JSON.stringify(clean));
}

/** Keep an in-app route trail that Escape can peel back safely without ever leaving Duskytunes. */
export function rememberRoute(url: URL): void {
	const current = routeKey(url);
	let trail = readTrail();
	const existing = trail.lastIndexOf(current);

	if (existing >= 0) trail = trail.slice(0, existing + 1);
	else trail.push(current);

	writeTrail(trail);
	if (current !== '/') safeSet(LAST_ROUTE, current);
}

/** Return one route layer toward Home and mutate the trail so repeated Escape keeps peeling. */
export function peelRoute(current: URL | string): string | null {
	const here = routeKey(current);
	if (here === '/') {
		writeTrail(['/']);
		return null;
	}

	let trail = readTrail();
	const existing = trail.lastIndexOf(here);
	if (existing >= 0) trail = trail.slice(0, existing + 1);
	else trail.push(here);

	trail.pop();
	const target = trail.at(-1) ?? '/';
	writeTrail(trail.length ? trail : ['/']);
	return target || '/';
}

export function lastRoute(): string | null {
	return safeGet(LAST_ROUTE);
}

export function saveRouteScroll(url: URL, top: number): void {
	safeSet(`${SCROLL_PREFIX}${routeKey(url)}`, String(Math.max(0, Math.round(top))));
}

export function loadRouteScroll(url: URL): number {
	const value = Number(safeGet(`${SCROLL_PREFIX}${routeKey(url)}`) ?? '0');
	return Number.isFinite(value) ? Math.max(0, value) : 0;
}
