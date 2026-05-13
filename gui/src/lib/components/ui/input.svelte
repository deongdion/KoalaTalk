<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils';

	type Props = HTMLInputAttributes & { class?: string };
	let { class: className, value = $bindable(), ...rest }: Props = $props();
</script>

<input
	bind:value
	class={cn(
		'flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-xs transition-[color,box-shadow] outline-none',
		'placeholder:text-muted-foreground/70',
		'focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/30',
		'disabled:cursor-not-allowed disabled:opacity-50',
		'file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium',
		className
	)}
	{...rest}
/>
