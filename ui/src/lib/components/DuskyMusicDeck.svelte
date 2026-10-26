<script lang="ts">
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import {
		PreviousIcon,
		NextIcon,
		PlayIcon,
		PauseIcon,
		Queue01Icon,
		MusicNote01Icon
	} from '@hugeicons/core-free-icons';
	import * as api from '$lib/api';
	import { np, playback } from '$lib/player.svelte';
	import { thumb } from '$lib/thumb';
	import { artworkAccent } from '$lib/artcolor';

	const art = $derived(playback.now?.thumbnail ? thumb(playback.now.thumbnail, 384) : '');
	let artFailed = $state(false);
	let artReady = $state(false);
	$effect(() => {
		art;
		artFailed = false;
		artReady = false;
	});
	let artAccent = $state<string | null>(null);
	let accentRun = 0;
	$effect(() => {
		const url = art;
		const run = ++accentRun;
		artAccent = null;
		if (!url) return;
		artworkAccent(url).then((color) => {
			if (run === accentRun) artAccent = color;
		});
	});
	const progress = $derived(
		playback.duration > 0 ? Math.min(100, Math.max(0, (playback.position / playback.duration) * 100)) : 0
	);
	const queueLeft = $derived(Math.max(0, playback.queue.items.length - playback.queue.currentIndex - 1));
	const sourceLabel = $derived(
		!playback.now
			? 'READY'
			: playback.now.videoId.startsWith('LOCAL:')
				? 'LOCAL'
				: playback.now.streamClient === 'cache'
					? 'CACHE'
					: 'STREAM'
	);

	// A low-cost activity meter. It advances from the real playback clock instead of running a
	// permanent CSS animation, so it sleeps when playback stops and costs no separate timer.
	const meterLevels = $derived.by(() => {
		if (!playback.now || playback.paused) return Array(12).fill(0.12) as number[];
		const phase = Math.floor(playback.position * 4);
		return Array.from({ length: 12 }, (_, i) => {
			const a = Math.sin(phase * 0.79 + i * 1.73);
			const b = Math.sin(phase * 0.47 + i * 2.31 + 1.2);
			return Math.min(0.96, 0.18 + Math.abs(a * 0.46 + b * 0.28));
		});
	});

	const fmt = (secs: number) => {
		if (!Number.isFinite(secs) || secs <= 0) return '0:00';
		const total = Math.floor(secs);
		const m = Math.floor(total / 60);
		const s = total % 60;
		return `${m}:${s.toString().padStart(2, '0')}`;
	};

	function openPlayer(tab?: 'queue' | 'lyrics') {
		if (tab) np.tab = tab;
		np.open = true;
	}
</script>

<section class="ryo-music-deck" style={artAccent ? `--ryo-art-accent:${artAccent}` : undefined} aria-label="Listening console">
	<div class="ryo-music-deck-art">
		{#if art && !artFailed}
			{#key art}
				<button type="button" class="ryo-music-deck-cover" onclick={() => openPlayer()} aria-label="Open now playing">
					<img src={art} alt="" draggable="false" data-loaded={artReady ? 'true' : 'false'} onload={() => (artReady = true)} onerror={() => (artFailed = true)} />
					<span class="ryo-music-deck-cover-grid" aria-hidden="true"></span>
				</button>
			{/key}
		{:else}
			<div class="ryo-music-deck-idle" aria-hidden="true">
				<div class="ryo-music-deck-orbit"></div>
				<HugeiconsIcon icon={MusicNote01Icon} class="h-7 w-7" />
			</div>
		{/if}

		<div class="ryo-music-deck-wave {playback.paused || !playback.now ? 'is-paused' : ''}" aria-hidden="true">
			{#each meterLevels as level, i (i)}
				<span style="--level:{level.toFixed(3)}"></span>
			{/each}
		</div>
		<div class="ryo-music-deck-art-mark" aria-hidden="true">// LIVE</div>
	</div>

	<div class="ryo-music-deck-copy">
		<div class="ryo-music-deck-running">
			<span>PLAYBACK</span><i></i><b>{sourceLabel}</b>
		</div>

		{#if playback.now}
			<button type="button" class="ryo-music-deck-track" title={playback.now.title} onclick={() => openPlayer()}>
				<strong>{playback.now.title}</strong>
				<span>{playback.now.artists}</span>
			</button>

			<div class="ryo-music-deck-readouts">
				<div><span>QUEUE</span><strong>{queueLeft}</strong></div>
				<div><span>LEVEL</span><strong>{Math.round(playback.volume)}%</strong></div>
				<div><span>SPEED</span><strong>{playback.speed.toFixed(2)}×</strong></div>
			</div>

			<div class="ryo-music-deck-progress" aria-label="Playback progress">
				<div><span>{fmt(playback.position)}</span><i><b style="width:{progress}%"></b></i><span>{fmt(playback.duration)}</span></div>
			</div>

			<div class="ryo-music-deck-actions">
				<button type="button" onclick={() => api.prevTrack()} aria-label="Previous track">
					<HugeiconsIcon icon={PreviousIcon} class="h-4 w-4" />
				</button>
				<button type="button" class="ryo-music-deck-play" onclick={() => api.togglePause()} aria-label={playback.paused ? 'Play' : 'Pause'}>
					{#if playback.paused}
						<HugeiconsIcon icon={PlayIcon} class="h-4 w-4" />
					{:else}
						<HugeiconsIcon icon={PauseIcon} class="h-4 w-4" />
					{/if}
				</button>
				<button type="button" onclick={() => api.nextTrack()} aria-label="Next track">
					<HugeiconsIcon icon={NextIcon} class="h-4 w-4" />
				</button>
				<button type="button" class="ryo-music-deck-queue" onclick={() => openPlayer('queue')} aria-label="Open queue">
					<HugeiconsIcon icon={Queue01Icon} class="h-4 w-4" />
					<span>QUEUE</span>
				</button>
			</div>

		{:else}
			<div class="ryo-music-deck-empty">
				<div class="ryo-music-deck-kanji">音</div>
				<strong>Ready to listen.</strong>
				<p>Search, open your library, or pick up a recent session.</p>
			</div>
		{/if}
	</div>
</section>
