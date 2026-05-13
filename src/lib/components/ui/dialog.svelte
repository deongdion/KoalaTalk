<script lang="ts">
	import type { Snippet } from 'svelte';
	import { X } from '@lucide/svelte';
	import { fade, scale } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { cn } from '$lib/utils';

	type Props = {
		open: boolean;
		onOpenChange?: (open: boolean) => void;
		class?: string;
		children?: Snippet;
	};
	let { open = $bindable(false), onOpenChange, class: className, children }: Props = $props();

	function close() {
		open = false;
		onOpenChange?.(false);
	}
	function onKey(e: KeyboardEvent) {
		if (e.key === 'Escape' && open) close();
	}

	$effect(() => {
		if (open) {
			document.body.classList.add('lock-scroll');
			return () => document.body.classList.remove('lock-scroll');
		}
	});
</script>

<svelte:window on:keydown={onKey} />

{#if open}
	<div class="fixed inset-0 z-50 flex items-center justify-center p-4">
		<button
			type="button"
			aria-label="닫기"
			onclick={close}
			class="absolute inset-0 bg-black/50 backdrop-blur-[2px]"
			transition:fade={{ duration: 160, easing: cubicOut }}
		></button>
		<div
			role="dialog"
			aria-modal="true"
			class={cn(
				'relative z-10 grid w-full max-w-lg gap-4 rounded-lg border bg-background p-6 shadow-xl',
				className
			)}
			transition:scale={{ duration: 180, start: 0.96, opacity: 0, easing: cubicOut }}
		>
			{@render children?.()}
			<button
				type="button"
				onclick={close}
				class="absolute top-4 right-4 rounded-sm opacity-70 transition-opacity hover:opacity-100"
				aria-label="닫기"
			>
				<X class="size-4" />
			</button>
		</div>
	</div>
{/if}
