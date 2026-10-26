<script lang="ts">
	// Lightweight anchored popover; it reads the same ranked local
	// search engine as the rest of Duskytunes and closes after an add so Home never sits under a stale
	// library picker. Scroll/resize repositioning is event-driven; there is no animation loop.
	import { onMount } from 'svelte';
	import { fade } from 'svelte/transition';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import { Cancel01Icon, Tick02Icon, Add01Icon, Search01Icon } from '@hugeicons/core-free-icons';
	import { thumb } from '$lib/thumb';
	import { addPick, library, loadLibrary, personal } from '$lib/player.svelte';
	import { mergeSaved } from '$lib/personal';
	import { indexCards, match } from '$lib/localsearch';

	let { onclose, anchor }: { onclose: () => void; anchor: HTMLElement | null } = $props();
	let filter = $state('');
	let panel: HTMLDivElement;
	let input: HTMLInputElement;
	let left = $state(16);
	let top = $state(16);
	let maxHeight = $state(440);

	const candidates = $derived(mergeSaved(personal, library.items, 'playlist'));
	const candidateIndex = $derived(indexCards(candidates));
	const matches = $derived.by(() => filter.trim() ? match(candidateIndex, filter) : candidates);
	const already = (id: string) => personal.picks.some((p) => p.id === id);

	function place() {
		if (!anchor || !panel) return;
		const a = anchor.getBoundingClientRect();
		const margin = 12;
		const gap = 8;
		const width = Math.min(360, window.innerWidth - margin * 2);
		const panelHeight = Math.min(panel.scrollHeight || 440, Math.max(240, window.innerHeight - margin * 2));
		const below = window.innerHeight - a.bottom - margin;
		const above = a.top - margin;
		left = Math.max(margin, Math.min(a.left, window.innerWidth - width - margin));
		if (below >= Math.min(300, panelHeight) || below >= above) {
			top = Math.min(a.bottom + gap, window.innerHeight - margin - Math.min(panelHeight, below));
			maxHeight = Math.max(220, below - gap);
		} else {
			maxHeight = Math.max(220, above - gap);
			top = Math.max(margin, a.top - gap - Math.min(panelHeight, maxHeight));
		}
	}

	function add(item: (typeof candidates)[number]) {
		if (already(item.id)) return;
		addPick(item);
		onclose();
	}

	loadLibrary();
	onMount(() => {
		const refresh = () => requestAnimationFrame(place);
		refresh();
		input?.focus();
		window.addEventListener('resize', refresh, { passive: true });
		document.addEventListener('scroll', refresh, true);
		return () => {
			window.removeEventListener('resize', refresh);
			document.removeEventListener('scroll', refresh, true);
		};
	});
	$effect(() => { filter; queueMicrotask(place); });
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<button type="button" class="fixed inset-0 z-[64] cursor-default bg-transparent" aria-label="Close shortcut picker" onclick={onclose}></button>
<div
	bind:this={panel}
	transition:fade={{ duration: 80 }}
	class="ryo-shortcut-picker fixed z-[65] flex w-[22.5rem] max-w-[calc(100vw-24px)] flex-col"
	style={`left:${left}px;top:${top}px;max-height:${maxHeight}px`}
	role="dialog"
	data-ryo-escape-owner
	aria-label="Add a shortcut"
>
	<div class="ryo-shortcut-picker-head">
		<div><span>// SHORTCUTS</span><strong>Add a shortcut</strong></div>
		<button type="button" onclick={onclose} aria-label="Close"><HugeiconsIcon icon={Cancel01Icon} class="h-3.5 w-3.5" /></button>
	</div>
	<label class="ryo-shortcut-picker-search">
		<HugeiconsIcon icon={Search01Icon} class="h-3.5 w-3.5" />
		<input bind:this={input} bind:value={filter} placeholder="Search playlists…" autocomplete="off" spellcheck="false" />
	</label>
	{#if library.loading && !library.items.length}
		<p class="ryo-shortcut-picker-empty">Loading library…</p>
	{:else if matches.length}
		<div class="ryo-shortcut-picker-list" data-ryo-own-scroll>
			{#each matches as item (item.id)}
				{@const on = already(item.id)}
				<button type="button" class="ryo-shortcut-picker-row" class:already={on} disabled={on} onclick={() => add(item)}>
					{#if item.thumbnail}
						<img src={thumb(item.thumbnail, 96)} alt="" loading="lazy" decoding="async" />
					{:else}
						<span class="ryo-shortcut-picker-placeholder">力</span>
					{/if}
					<span class="ryo-shortcut-picker-copy"><strong>{item.title}</strong><small>{item.subtitle ?? 'Playlist'}</small></span>
					<span class="ryo-shortcut-picker-action"><HugeiconsIcon icon={on ? Tick02Icon : Add01Icon} class="h-3.5 w-3.5" /></span>
				</button>
			{/each}
		</div>
	{:else}
		<p class="ryo-shortcut-picker-empty">Nothing matches “{filter.trim()}”.</p>
	{/if}
</div>
