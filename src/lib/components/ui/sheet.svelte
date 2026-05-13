<script lang="ts">
	import type { Snippet } from 'svelte';
	import { X } from '@lucide/svelte';
	import { fade, fly } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { cn } from '$lib/utils';

	type Props = {
		open: boolean;
		onOpenChange?: (open: boolean) => void;
		side?: 'right' | 'left';
		class?: string;
		title?: string;
		children?: Snippet;
		actions?: Snippet;
		footer?: Snippet;
	};
	let {
		open = $bindable(false),
		onOpenChange,
		side = 'right',
		class: className,
		title,
		children,
		actions,
		footer
	}: Props = $props();

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

	const flyX = side === 'right' ? 24 : -24;
</script>

<svelte:window on:keydown={onKey} />

{#if open}
	<div class="fixed inset-0 z-[55]">
		<button
			type="button"
			aria-label="닫기"
			onclick={close}
			class="absolute inset-0 bg-black/40 backdrop-blur-[2px]"
			transition:fade={{ duration: 160, easing: cubicOut }}
		></button>
		<aside
			role="dialog"
			aria-modal="true"
			class={cn(
				'absolute top-0 flex h-full w-full max-w-md flex-col border-border bg-background shadow-xl',
				side === 'right' ? 'right-0 border-l' : 'left-0 border-r',
				className
			)}
			transition:fly={{ x: flyX, duration: 220, easing: cubicOut, opacity: 0.4 }}
		>
			<header class="flex h-14 shrink-0 items-center justify-between border-b px-5">
				<h2 class="text-sm font-semibold">{title ?? ''}</h2>
				<div class="flex items-center gap-1">
					{@render actions?.()}
					<button
						type="button"
						onclick={close}
						class="flex size-7 items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-accent-foreground"
						aria-label="닫기"
					>
						<X class="size-4" />
					</button>
				</div>
			</header>
			<div class="min-h-0 flex-1 overflow-y-auto">
				{@render children?.()}
			</div>
			{#if footer}
				<footer class="shrink-0 border-t px-5 py-3">
					{@render footer()}
				</footer>
			{/if}
		</aside>
	</div>
{/if}
