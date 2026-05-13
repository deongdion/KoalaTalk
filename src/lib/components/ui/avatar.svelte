<script lang="ts">
	import { cn } from '$lib/utils';

	type Props = {
		src?: string;
		alt?: string;
		fallback?: string;
		class?: string;
		size?: number;
	};

	let { src, alt = '', fallback = '', class: className, size = 40 }: Props = $props();

	let loaded = $state(false);
	let errored = $state(false);
</script>

<div
	class={cn('relative inline-flex shrink-0 overflow-hidden bg-muted', className)}
	style="width: {size}px; height: {size}px; clip-path: url(#kt-squircle); -webkit-clip-path: url(#kt-squircle);"
>
	{#if src && !errored}
		<img
			{src}
			{alt}
			class={cn(
				'aspect-square size-full object-cover transition-opacity',
				loaded ? 'opacity-100' : 'opacity-0'
			)}
			onload={() => (loaded = true)}
			onerror={() => (errored = true)}
		/>
	{/if}
	{#if !src || errored || !loaded}
		<span
			class="absolute inset-0 flex items-center justify-center text-xs font-semibold text-muted-foreground select-none"
		>
			{fallback}
		</span>
	{/if}
</div>
