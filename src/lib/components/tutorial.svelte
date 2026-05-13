<script lang="ts" module>
	export type TourStep = {
		title: string;
		body: string;
		hint?: string;
		selector?: string;
		/** when false, the spotlight is non-interactive (blocks clicks on the underlying element).
		 *  default: true */
		interactive?: boolean;
	};
</script>

<script lang="ts">
	import { onMount } from 'svelte';
	import { fade, scale } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import Button from './ui/button.svelte';
	import { cn } from '$lib/utils';

	type Props = {
		open: boolean;
		steps: TourStep[];
		onClose?: () => void;
	};

	let { open = $bindable(false), steps, onClose }: Props = $props();

	let step = $state(0);
	let targetRect = $state<{ top: number; left: number; width: number; height: number } | null>(
		null
	);
	let tipRect = $state<{ top: number; left: number; placement: 'below' | 'above' | 'center' }>(
		{ top: 0, left: 0, placement: 'center' }
	);
	let tipEl = $state<HTMLDivElement | null>(null);

	const PAD = 6;
	const GAP = 14;
	const MARGIN = 16;

	function next() {
		if (step < steps.length - 1) step += 1;
		else end();
	}
	function prev() {
		if (step > 0) step -= 1;
	}
	function end() {
		open = false;
		onClose?.();
	}

	function recompute() {
		if (!open) return;
		const sel = steps[step]?.selector;
		const el = sel ? (document.querySelector(sel) as HTMLElement | null) : null;
		if (!el) {
			targetRect = null;
			const W = window.innerWidth;
			const H = window.innerHeight;
			const tw = tipEl?.offsetWidth ?? 360;
			const th = tipEl?.offsetHeight ?? 220;
			tipRect = {
				top: Math.max(MARGIN, (H - th) / 2),
				left: Math.max(MARGIN, (W - tw) / 2),
				placement: 'center'
			};
			return;
		}

		const r = el.getBoundingClientRect();
		const visible =
			r.top >= 0 && r.bottom <= window.innerHeight && r.left >= 0 && r.right <= window.innerWidth;
		if (!visible) {
			el.scrollIntoView({ block: 'center', behavior: 'smooth' });
		}
		const rect = el.getBoundingClientRect();
		targetRect = {
			top: rect.top - PAD,
			left: rect.left - PAD,
			width: rect.width + PAD * 2,
			height: rect.height + PAD * 2
		};

		const W = window.innerWidth;
		const H = window.innerHeight;
		const tw = tipEl?.offsetWidth ?? 360;
		const th = tipEl?.offsetHeight ?? 220;

		let placement: 'below' | 'above' | 'center' = 'below';
		let top = rect.bottom + GAP;
		if (top + th + MARGIN > H) {
			if (rect.top - GAP - th - MARGIN > 0) {
				placement = 'above';
				top = rect.top - GAP - th;
			} else {
				placement = 'center';
				top = Math.max(MARGIN, (H - th) / 2);
			}
		}
		let left = rect.left + rect.width / 2 - tw / 2;
		left = Math.max(MARGIN, Math.min(W - tw - MARGIN, left));
		tipRect = { top, left, placement };
	}

	$effect(() => {
		// re-run when open / step changes
		void open;
		void step;
		if (!open) return;
		// reset to first step on open
		const id = setTimeout(() => {
			recompute();
			// second pass after content has been measured
			requestAnimationFrame(recompute);
		}, 30);

		const onChange = () => recompute();
		window.addEventListener('resize', onChange);
		window.addEventListener('scroll', onChange, true);
		const ro = new ResizeObserver(onChange);
		ro.observe(document.body);

		return () => {
			clearTimeout(id);
			window.removeEventListener('resize', onChange);
			window.removeEventListener('scroll', onChange, true);
			ro.disconnect();
		};
	});

	$effect(() => {
		if (open) {
			step = 0;
			document.body.classList.add('lock-scroll');
			return () => document.body.classList.remove('lock-scroll');
		}
	});

	function onKey(e: KeyboardEvent) {
		if (!open) return;
		if (e.key === 'Escape') end();
		if (e.key === 'ArrowRight') next();
		if (e.key === 'ArrowLeft') prev();
	}
</script>

<svelte:window on:keydown={onKey} />

{#if open}
	{@const cur = steps[step]}
	<!-- wrapper is click-through; only masks + tooltip catch events -->
	<div
		class="pointer-events-none fixed inset-0 z-[60]"
		transition:fade={{ duration: 160, easing: cubicOut }}
	>
		<!-- darkened mask: 4 panels around the target so the spotlight area stays interactive -->
		{#if targetRect}
			<div
				class="pointer-events-auto absolute bg-black/55 backdrop-blur-[2px]"
				style="top:0; left:0; right:0; height:{Math.max(0, targetRect.top)}px;"
			></div>
			<div
				class="pointer-events-auto absolute bg-black/55 backdrop-blur-[2px]"
				style="top:{targetRect.top + targetRect.height}px; left:0; right:0; bottom:0;"
			></div>
			<div
				class="pointer-events-auto absolute bg-black/55 backdrop-blur-[2px]"
				style="top:{targetRect.top}px; height:{targetRect.height}px; left:0; width:{Math.max(
					0,
					targetRect.left
				)}px;"
			></div>
			<div
				class="pointer-events-auto absolute bg-black/55 backdrop-blur-[2px]"
				style="top:{targetRect.top}px; height:{targetRect.height}px; left:{targetRect.left +
					targetRect.width}px; right:0;"
			></div>

			<!-- highlight ring (non-interactive) -->
			<div
				class="pointer-events-none absolute rounded-md ring-2 ring-primary"
				style="top:{targetRect.top}px; left:{targetRect.left}px; width:{targetRect.width}px; height:{targetRect.height}px; box-shadow: 0 0 0 4px color-mix(in oklch, var(--primary) 25%, transparent);"
			></div>

			{#if cur.interactive === false}
				<!-- click blocker: prevents underlying button from being triggered; advances on click -->
				<button
					type="button"
					aria-label="다음 단계로"
					onclick={next}
					class="pointer-events-auto absolute cursor-pointer rounded-md"
					style="top:{targetRect.top}px; left:{targetRect.left}px; width:{targetRect.width}px; height:{targetRect.height}px;"
				></button>
			{/if}
		{:else}
			<div class="pointer-events-auto absolute inset-0 bg-black/55 backdrop-blur-[2px]"></div>
		{/if}

		<!-- tooltip card -->
		<div
			bind:this={tipEl}
			class="pointer-events-auto absolute w-[360px] rounded-lg border bg-card shadow-xl"
			style="top:{tipRect.top}px; left:{tipRect.left}px;"
			transition:scale={{ duration: 180, start: 0.96, opacity: 0, easing: cubicOut }}
		>
			<div class="p-5">
				<div class="mb-3 flex items-center gap-3">
					<span class="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
						STEP {step + 1} / {steps.length}
					</span>
					<div class="ml-auto flex gap-1">
						{#each steps as _, i}
							<span
								class={cn(
									'h-1 w-5 rounded-full transition-colors',
									i <= step ? 'bg-primary' : 'bg-muted'
								)}
							></span>
						{/each}
					</div>
				</div>
				<h3 class="text-base font-semibold leading-none">{cur.title}</h3>
				<p class="mt-3 text-sm leading-relaxed text-muted-foreground">{cur.body}</p>
				{#if cur.hint}
					<p class="mt-3 rounded-md border border-dashed bg-muted/40 p-2 text-[11px] text-foreground">
						💡 {cur.hint}
					</p>
				{/if}
			</div>
			<div class="flex items-center justify-between gap-2 border-t px-5 py-3">
				<Button variant="ghost" size="sm" onclick={end}>확인 했습니다</Button>
				<div class="flex gap-2">
					<Button variant="outline" size="sm" disabled={step === 0} onclick={prev}>이전</Button>
					{#if step < steps.length - 1}
						<Button size="sm" onclick={next}>다음</Button>
					{:else}
						<Button size="sm" onclick={end}>완료</Button>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}
