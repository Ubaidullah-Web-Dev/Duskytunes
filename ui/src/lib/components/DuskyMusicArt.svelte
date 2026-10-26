<script lang="ts">
	let { mode = 'library', compact = false }: { mode?: 'search' | 'library' | 'data' | 'about'; compact?: boolean } = $props();
	const portrait = $derived(mode === 'data' || mode === 'about');
	const viewBox = $derived(portrait ? '0 0 360 500' : '0 0 720 240');
</script>

<div class="ryo-music-art ryo-music-art-{mode}" class:compact class:portrait aria-hidden="true">
	<svg {viewBox} role="presentation" preserveAspectRatio="xMidYMid meet">
		<defs>
			<pattern id="grid-{mode}" width="30" height="30" patternUnits="userSpaceOnUse">
				<path d="M30 0H0V30" class="gridline" />
			</pattern>
			<pattern id="hatch-{mode}" width="9" height="9" patternUnits="userSpaceOnUse" patternTransform="rotate(35)">
				<path d="M0 0V9" class="hatchline" />
			</pattern>
		</defs>
		<rect class="paper" width="100%" height="100%" />
		<rect class="grid" width="100%" height="100%" fill="url(#grid-{mode})" />

		{#if mode === 'search'}
			<g class="search-plate">
				<path class="signal" d="M38 136H74C90 136 90 73 106 73s17 126 34 126s18-91 35-91s18 56 35 56s19-28 39-28h86" />
				<path class="signal faint" d="M38 160h48c20 0 22-37 43-37s25 62 47 62s25-39 50-39s27 26 61 26h48" />
				<circle class="scope" cx="552" cy="121" r="72" /><circle class="scope inner" cx="552" cy="121" r="42" />
				<path class="scope-cross" d="M552 42v158M473 121h158" /><path class="needle" d="M552 121l48-39" /><circle class="scope-dot" cx="600" cy="82" r="5" />
				<text class="kanji" x="517" y="145">探</text>
				<text class="micro" x="28" y="29">// SIGNAL / DISCOVERY</text><text class="micro right" x="692" y="29">QUERY-01</text>
				<text class="micro" x="28" y="218">FIND THE FREQUENCY · INDEX / MUSIC</text>
			</g>
		{:else if mode === 'library'}
			<g class="library-plate">
				<circle class="vinyl" cx="172" cy="121" r="86" /><circle class="groove" cx="172" cy="121" r="69" /><circle class="groove" cx="172" cy="121" r="53" /><circle class="groove" cx="172" cy="121" r="36" />
				<circle class="label" cx="172" cy="121" r="23" /><circle class="hole" cx="172" cy="121" r="3" />
				<path class="arm" d="M319 41L294 58L245 147" /><path class="arm-head" d="M236 145l22 10l-12 24l-22-10z" />
				<g class="spectrum"><rect x="390" y="145" width="9" height="52"/><rect x="409" y="119" width="9" height="78"/><rect x="428" y="91" width="9" height="106"/><rect x="447" y="127" width="9" height="70"/><rect x="466" y="72" width="9" height="125"/><rect x="485" y="105" width="9" height="92"/><rect x="504" y="85" width="9" height="112"/><rect x="523" y="135" width="9" height="62"/></g>
				<text class="kanji large" x="388" y="68">収蔵</text>
				<text class="micro" x="28" y="29">// ARCHIVE / 33⅓</text><text class="micro right" x="692" y="29">COLLECTION-02</text>
				<text class="micro" x="389" y="218">PLAYLIST · ALBUM · ARTIST · LOCAL</text>
			</g>
		{:else if mode === 'data'}
			<g class="data-plate">
				<text class="micro" x="24" y="30">// CACHE / TRANSPORT</text><text class="micro right" x="336" y="30">DATA-03</text>
				<rect class="cassette" x="38" y="73" width="284" height="226" rx="4" />
				<circle class="reel" cx="112" cy="165" r="37"/><circle class="reel" cx="248" cy="165" r="37"/><circle class="reel-hole" cx="112" cy="165" r="12"/><circle class="reel-hole" cx="248" cy="165" r="12"/>
				<path class="tape" d="M112 165C147 205 213 205 248 165"/><path class="cassette-slot" d="M86 266h188l-17-34H103z"/>
				<g transform="translate(43 330)"><rect class="meter" width="274" height="10"/><rect class="meter fill" width="185" height="10"/><rect class="meter" y="31" width="274" height="10"/><rect class="meter fill small" y="31" width="112" height="10"/><rect class="meter" y="62" width="274" height="10"/><rect class="meter fill mid" y="62" width="228" height="10"/></g>
				<text class="kanji portrait-mark" x="238" y="458">保存</text>
			</g>
		{:else}
			<g class="about-plate">
				<text class="micro" x="24" y="30">// INSTRUMENT / BUILD</text><text class="micro right" x="336" y="30">INFO-04</text>
				<path class="headphone" d="M72 238v-60c0-86 48-132 108-132s108 46 108 132v60" />
				<rect class="ear" x="48" y="206" width="52" height="104" rx="22"/><rect class="ear" x="260" y="206" width="52" height="104" rx="22"/>
				<path class="wave" d="M42 365h43l13-42l17 83l19-119l22 142l20-90l23 50l21-24h98" />
				<circle class="dial" cx="180" cy="365" r="104" />
				<text class="kanji portrait-mark" x="145" y="204">力</text>
				<text class="micro" x="45" y="473">RUST · MPV · WEBKITGTK · DUSKY</text>
			</g>
		{/if}
	</svg>
	<div class="ryo-music-art-corners"><i></i><i></i><i></i><i></i></div>
</div>
