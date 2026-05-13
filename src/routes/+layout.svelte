<script lang="ts">
	import '../app.css';
	let { children } = $props();
</script>

<!-- global SVG defs (squircle clip path used by Avatar) -->
<svg width="0" height="0" aria-hidden="true" style="position:absolute">
	<defs>
		<clipPath id="kt-squircle" clipPathUnits="objectBoundingBox">
			<path
				d="M0.5,1 C0.131946,1 0,0.868063 0,0.5 C0,0.145830 0.131946,0 0.5,0 C0.868063,0 1,0.131946 1,0.5 C1,0.868063 0.868063,1 0.5,1 Z"
			/>
		</clipPath>
	</defs>
</svg>

{@render children()}
