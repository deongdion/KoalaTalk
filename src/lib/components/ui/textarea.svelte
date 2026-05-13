<script lang="ts">
	import type { HTMLTextareaAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils';

	type Props = HTMLTextareaAttributes & { class?: string };
	let { class: className, value = $bindable(), ...rest }: Props = $props();
</script>

<textarea
	bind:value
	class={cn(
		'flex w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-xs outline-none transition-[color,box-shadow]',
		'placeholder:text-muted-foreground/70',
		'focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/30',
		'disabled:cursor-not-allowed disabled:opacity-50',
		'field-sizing-content min-h-16 resize-none',
		className
	)}
	{...rest}
></textarea>
