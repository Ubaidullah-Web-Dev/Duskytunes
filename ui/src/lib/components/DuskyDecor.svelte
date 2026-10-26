<script lang="ts">
	let {
		caption = 'Every track this machine can carry — browse the collection, then commit.',
		title = '音楽',
		sub = '再生',
		tate = '音を選ぶ',
		code = 'DUSKY · TUNES',
		seal = '音',
		image = '',
		readout = []
	}: {
		caption?: string;
		title?: string;
		sub?: string;
		tate?: string;
		code?: string;
		seal?: string;
		image?: string;
		readout?: string[];
	} = $props();
</script>

<div class="ryo-decor" aria-hidden="true">
	<div class="ryo-decor-art {image ? 'ryo-decor-art-image' : ''}">
		{#if image}
			{#key image}
				<img src={image} alt="" class="ryo-decor-image" draggable="false" />
			{/key}
			<div class="ryo-decor-image-grid"></div>
			<svg class="ryo-waveform ryo-waveform-over-art" viewBox="0 0 520 120" preserveAspectRatio="none">
				<path class="ryo-wave-line ryo-wave-b" d="M0 72 C34 72,38 35,68 35 S98 88,128 88 S156 40,188 40 S220 83,250 83 S278 35,308 35 S338 92,368 92 S396 43,426 43 S456 80,486 80 S506 68,520 68" />
			</svg>
		{:else}
			<svg class="ryo-waveform" viewBox="0 0 520 120" preserveAspectRatio="none">
				<g class="ryo-gridlines">
					<path d="M0 30H520 M0 60H520 M0 90H520" />
					<path d="M65 0V120 M130 0V120 M195 0V120 M260 0V120 M325 0V120 M390 0V120 M455 0V120" />
				</g>
				<path class="ryo-wave-line ryo-wave-a" d="M0 60 C24 60,28 105,52 105 S82 18,108 18 S136 108,164 108 S192 16,220 16 S248 103,276 103 S302 20,330 20 S354 99,380 99 S407 27,432 27 S460 88,486 88 S506 60,520 60" />
				<path class="ryo-wave-line ryo-wave-b" d="M0 72 C34 72,38 35,68 35 S98 88,128 88 S156 40,188 40 S220 83,250 83 S278 35,308 35 S338 92,368 92 S396 43,426 43 S456 80,486 80 S506 68,520 68" />
			</svg>
		{/if}
		<div class="ryo-art-ticks ryo-tick-a"></div>
		<div class="ryo-art-ticks ryo-tick-b"></div>
	</div>

	<div class="ryo-decor-copy">
		<div class="ryo-decor-code">// {code}</div>
		<p class="ryo-decor-caption">{caption}</p>
		{#if readout.length}
			<div class="ryo-decor-readout">
				{#each readout.slice(0, 3) as cell}
					{@const parts = cell.split('|')}
					<div class="ryo-readout-cell">
						<span>{parts[0]}</span>
						<strong>{parts[1] ?? '—'}</strong>
					</div>
				{/each}
			</div>
		{/if}
		<div class="ryo-decor-title">{title}</div>
		<div class="ryo-decor-sub">{sub}</div>
		<div class="ryo-barcode"></div>
		<div class="ryo-barcode-label">DUSKY · TUNES</div>
		<div class="ryo-seal">{seal}</div>
		<div class="ryo-tate">{tate}</div>
	</div>
</div>
