<script lang="ts">
	import { Check } from '@lucide/svelte';
	import { cn } from '$lib/utils';

	type Props = {
		checked?: boolean;
		indeterminate?: boolean;
		disabled?: boolean;
		class?: string;
		onCheckedChange?: (next: boolean) => void;
	};
	let {
		checked = $bindable(false),
		indeterminate = false,
		disabled = false,
		class: className,
		onCheckedChange
	}: Props = $props();

	function toggle() {
		if (disabled) return;
		checked = !checked;
		onCheckedChange?.(checked);
	}
</script>

<button
	type="button"
	role="checkbox"
	aria-checked={indeterminate ? 'mixed' : checked}
	{disabled}
	onclick={toggle}
	class={cn(
		'peer flex size-4 shrink-0 items-center justify-center rounded-[4px] border border-input shadow-xs outline-none transition-colors',
		'focus-visible:ring-2 focus-visible:ring-ring/30 focus-visible:ring-offset-1',
		'disabled:cursor-not-allowed disabled:opacity-50',
		(checked || indeterminate) && 'border-primary bg-primary text-primary-foreground',
		className
	)}
>
	{#if indeterminate}
		<span class="block h-0.5 w-2 bg-current"></span>
	{:else if checked}
		<Check class="size-3" strokeWidth={3} />
	{/if}
</button>
