<script lang="ts">
	let { active = false }: { active?: boolean } = $props();
</script>

<div class="ryo-atmosphere" class:ryo-atmosphere-active={active} aria-hidden="true">
	<svg viewBox="0 0 1600 900" preserveAspectRatio="none">
		<path class="ryo-atmo-line ryo-atmo-a" d="M-80 190 C140 110 250 310 470 232 S810 80 1030 210 S1370 370 1680 170" />
		<path class="ryo-atmo-line ryo-atmo-b" d="M-120 520 C90 390 290 600 520 500 S865 305 1110 470 S1420 655 1710 455" />
		<path class="ryo-atmo-line ryo-atmo-c" d="M-40 735 C190 650 355 820 610 710 S940 560 1190 690 S1450 830 1660 720" />
		<path class="ryo-atmo-hair" d="M120 0 V900 M460 0 V900 M880 0 V900 M1320 0 V900" />
	</svg>
</div>
