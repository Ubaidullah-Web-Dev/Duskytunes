/**
 * Dusky-flavoured scroll assistance for WebKitGTK.
 *
 * IMPORTANT: Wayland precision/touchpad scrolling already arrives as DOM_DELTA_PIXEL and WebKitGTK
 * handles it better than any JS imitation of Qt Flickable.  Never intercept those events — even a
 * fast two-finger swipe can have a delta larger than 40px, which is why the v3 magnitude heuristic
 * occasionally mistook touchpad input for a mouse notch and produced a jumpy target chase.
 *
 * Only legacy/coarse LINE/PAGE wheel events get a short velocity tail.  This is deliberately small:
 * the goal is to take the square edge off a mouse-wheel notch, not to replace the browser scroller.
 */
export function duskyWheelScroll(el: HTMLElement) {
	let raf = 0;
	let velocity = 0; // px per nominal 60 Hz frame
	let lastFrame = 0;

	const clamp = (v: number) => Math.max(0, Math.min(v, el.scrollHeight - el.clientHeight));

	function stopKinetic() {
		if (raf) cancelAnimationFrame(raf);
		raf = 0;
		velocity = 0;
		lastFrame = 0;
	}

	function kineticFrame(now: number) {
		if (!lastFrame) lastFrame = now - 16.667;
		const dt = Math.min(32, Math.max(4, now - lastFrame));
		lastFrame = now;
		const frameScale = dt / 16.667;

		const before = el.scrollTop;
		const next = clamp(before + velocity * frameScale);
		el.scrollTop = next;

		// Around a 170–230 ms useful tail at 60 Hz, close to Dusky's move/swap register.
		velocity *= Math.pow(0.80, frameScale);

		const hitBoundary = Math.abs(next - before) < 0.01 && Math.abs(velocity) > 0.35;
		if (Math.abs(velocity) < 0.35 || hitBoundary) {
			stopKinetic();
			return;
		}
		raf = requestAnimationFrame(kineticFrame);
	}

	function onWheel(e: WheelEvent) {
		if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.shiftKey) return;
		if (Math.abs(e.deltaY) <= Math.abs(e.deltaX)) return;

		// Pixel-mode is Wayland/libinput precision input. Leave it *entirely* native.
		if (e.deltaMode === WheelEvent.DOM_DELTA_PIXEL) {
			stopKinetic();
			return;
		}

		if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;

		e.preventDefault();
		const pixels =
			e.deltaMode === WheelEvent.DOM_DELTA_LINE
				? e.deltaY * 34
				: e.deltaY * el.clientHeight * 0.82;

		// Add an impulse instead of chasing a target. Repeated notches therefore build velocity like
		// Flickable rather than restarting an ease-to-target curve on every event.
		velocity = Math.max(-80, Math.min(80, velocity + pixels * 0.22));
		if (!raf) raf = requestAnimationFrame(kineticFrame);
	}

	el.addEventListener('wheel', onWheel, { passive: false });
	el.addEventListener('pointerdown', stopKinetic, { passive: true });
	el.addEventListener('touchstart', stopKinetic, { passive: true });
	window.addEventListener('keydown', stopKinetic, { passive: true });

	return () => {
		stopKinetic();
		el.removeEventListener('wheel', onWheel);
		el.removeEventListener('pointerdown', stopKinetic);
		el.removeEventListener('touchstart', stopKinetic);
		window.removeEventListener('keydown', stopKinetic);
	};
}


/**
 * WebKitGTK precision-scroll safety net.
 *
 * libinput touchpads arrive as DOM_DELTA_PIXEL wheel events. Normally WebKit scrolls the nearest
 * overflow container natively, which remains the preferred path. Some renderer/session combinations
 * can deliver the wheel event but fail to advance the scroller. We do not cancel the event: one
 * frame later, only if native scrolling made no progress, apply the accumulated pixel delta to the
 * exact nearest vertical scroller under the pointer. Nested queue/search/settings scrollers therefore
 * keep ownership of their gesture and the page behind them never double-scrolls.
 */
export function initPrecisionScrollFallback() {
	type Pending = { before: number; delta: number; raf: number };
	const pending = new Map<HTMLElement, Pending>();

	function isKnownVerticalScroller(el: HTMLElement) {
		return (
			el.hasAttribute('data-ryo-own-scroll') ||
			el.classList.contains('overflow-y-auto') ||
			el.classList.contains('overflow-y-scroll')
		);
	}

	function nearestVerticalScroller(e: WheelEvent): HTMLElement | null {
		for (const part of e.composedPath()) {
			if (!(part instanceof HTMLElement) || !isKnownVerticalScroller(part)) continue;
			if (part.scrollHeight > part.clientHeight + 1) return part;
		}
		return null;
	}

	function onWheel(e: WheelEvent) {
		if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.shiftKey) return;
		if (e.deltaMode !== WheelEvent.DOM_DELTA_PIXEL) return;
		if (Math.abs(e.deltaY) <= Math.abs(e.deltaX) || Math.abs(e.deltaY) < 0.01) return;

		const el = nearestVerticalScroller(e);
		if (!el) return;
		const max = Math.max(0, el.scrollHeight - el.clientHeight);
		if ((e.deltaY < 0 && el.scrollTop <= 0) || (e.deltaY > 0 && el.scrollTop >= max)) return;

		const existing = pending.get(el);
		if (existing) {
			existing.delta += e.deltaY;
			return;
		}

		const item: Pending = { before: el.scrollTop, delta: e.deltaY, raf: 0 };
		item.raf = requestAnimationFrame(() => {
			pending.delete(el);
			// Native WebKit scrolling won the race: leave it completely alone.
			if (Math.abs(el.scrollTop - item.before) >= 0.5) return;
			const limit = Math.max(0, el.scrollHeight - el.clientHeight);
			el.scrollTop = Math.max(0, Math.min(limit, item.before + item.delta));
		});
		pending.set(el, item);
	}

	window.addEventListener('wheel', onWheel, { passive: true });
	return () => {
		window.removeEventListener('wheel', onWheel);
		for (const item of pending.values()) cancelAnimationFrame(item.raf);
		pending.clear();
	};
}


/**
 * Hard scroll ownership for nested result panes on WebKitGTK.
 *
 * `overscroll-behavior` is not sufficient on every WebKitGTK/Wayland build: a precision gesture can
 * advance the inner scroller and still chain into the page. This attachment consumes only vertical
 * wheel/touchpad input while the pointer is over the nested pane and applies the original granular
 * delta directly, including at the boundary, so the page behind it never moves. It is deliberately
 * scoped to panes that opt in; normal page/touchpad scrolling remains native.
 */
export function ownNestedVerticalScroll(el: HTMLElement) {
	function onWheel(e: WheelEvent) {
		if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.shiftKey) return;
		if (Math.abs(e.deltaY) <= Math.abs(e.deltaX)) return;
		const max = Math.max(0, el.scrollHeight - el.clientHeight);
		if (max <= 0) return;
		const delta = e.deltaMode === WheelEvent.DOM_DELTA_LINE
			? e.deltaY * 34
			: e.deltaMode === WheelEvent.DOM_DELTA_PAGE
				? e.deltaY * el.clientHeight * 0.82
				: e.deltaY;
		e.preventDefault();
		e.stopPropagation();
		el.scrollTop = Math.max(0, Math.min(max, el.scrollTop + delta));
	}
	el.addEventListener('wheel', onWheel, { passive: false });
	return () => el.removeEventListener('wheel', onWheel);
}
