<script lang="ts">
	import { onMount } from 'svelte';
	import { fade, scale } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import {
		Sun,
		Moon,
		LogOut,
		Image as ImageIcon,
		Type,
		X,
		ChevronUp,
		ChevronDown,
		Plus,
		Play,
		AlertTriangle,
		Loader2,
		RefreshCw,
		Trash2,
		Settings,
		Send,
		Search,
		Terminal,
		Square
	} from '@lucide/svelte';

	import Button from '$lib/components/ui/button.svelte';
	import Input from '$lib/components/ui/input.svelte';
	import Textarea from '$lib/components/ui/textarea.svelte';
	import Avatar from '$lib/components/ui/avatar.svelte';
	import Badge from '$lib/components/ui/badge.svelte';
	import Card from '$lib/components/ui/card.svelte';
	import Checkbox from '$lib/components/ui/checkbox.svelte';
	import Dialog from '$lib/components/ui/dialog.svelte';
	import Sheet from '$lib/components/ui/sheet.svelte';
	import Tutorial, { type TourStep } from '$lib/components/tutorial.svelte';

	import {
		defaultSettings,
		loadSettings,
		saveSettings,
		type ExtractedChannel,
		type KakaoStatus,
		type MessageBlock,
		type Profile,
		type Settings as AppSettings
	} from '$lib/data';
	import {
		cancelWork,
		checkKakao,
		clearSentCache,
		exitApp,
		extractChannels,
		getSentCacheCount,
		listenAuthLost,
		listenLog,
		pickImageFile,
		runAutomation,
		sendToSelected,
		type LogPayload
	} from '$lib/tauri';
	import { cn } from '$lib/utils';

	// ─── state ───────────────────────────────────────────────────────
	let status = $state<KakaoStatus>('checking');
	let profile = $state<Profile | null>(null);
	let theme = $state<'light' | 'dark'>('light');
	let lastError = $state<string>('');

	let keywords = $state<string[]>(['카페']);
	let keywordDraft = $state('');

	let waitForSelection = $state(false);

	let blocks = $state<MessageBlock[]>([
		{ id: crypto.randomUUID(), kind: 'text', content: '안녕하세요! KoalaTalk 입니다.' }
	]);

	let extracted = $state<ExtractedChannel[]>([]);
	let extracting = $state(false);
	let running = $state(false);

	// address-based filter (applied to `extracted` on demand)
	let addressKeywords = $state<string[]>([]);
	let addressDraft = $state('');

	// logical selection (don't iterate 10k items on toggle-all)
	let selectMode = $state<'all' | 'manual'>('manual');
	let excludedSet = $state<Set<string>>(new Set());
	let includedSet = $state<Set<string>>(new Set());

	type LogLine = { id: string; level: LogPayload['level']; text: string };
	let log = $state<LogLine[]>([]);

	let settingsOpen = $state(false);
	let logSheetOpen = $state(false);
	let settings = $state<AppSettings>(structuredClone(defaultSettings));
	let cacheCount = $state(0);

	// ─── tutorial (demo only, unrelated to real settings) ─────────────
	const tutorialSteps: TourStep[] = [
		{
			title: '카카오톡 연결 상태',
			body: '여기서 카카오톡 PC가 실행 중인지 확인할 수 있어요. 녹색 점이 보이면 정상 연결, 회색이면 확인 중, 빨간색이면 미실행입니다.',
			selector: '[data-tour="status-badge"]'
		},
		{
			title: '내 프로필',
			body: '카카오톡에서 추출한 본인 정보가 여기에 표시됩니다. 이 정보가 보이면 토큰 추출이 정상적으로 완료된 거예요.',
			selector: '[data-tour="profile"]'
		},
		{
			title: '검색 키워드',
			body: '여기에 키워드를 입력하고 Enter 를 누르면 칩 형태로 추가됩니다. 한 번에 여러 키워드를 등록할 수 있어요.',
			hint: '한 번 입력해 보세요 — Enter 로 추가, Backspace 로 마지막 키워드 제거',
			selector: '[data-tour="keyword-input"]'
		},
		{
			title: '추출 후 채널 선택 대기',
			body: '체크박스를 켜면 "검색만" 수행하고, 결과 채널 중 직접 고른 것에만 메시지를 보낼 수 있어요. 끄면 즉시 모드로 모든 채널에 자동 전송됩니다.',
			hint: '직접 클릭해서 켜고 꺼보세요',
			selector: '[data-tour="wait-toggle"]'
		},
		{
			title: '채널 추출 버튼',
			body: '추출 모드일 때 활성화됩니다. 누르면 입력한 키워드별로 카카오톡 채널을 검색해 아래에 목록을 보여줘요. (튜토리얼 중에는 클릭이 비활성화되어 있어요.)',
			selector: '[data-tour="extract-btn"]',
			interactive: false
		},
		{
			title: '메시지 블럭',
			body: '텍스트나 이미지 블럭을 추가해 보낼 메시지를 구성하세요. 위/아래 화살표로 순서를 바꿀 수 있고, 채팅창에 한 번에 차례로 전송됩니다.',
			selector: '[data-tour="blocks-card"]'
		},
		{
			title: '실행 로그',
			body: '콘솔 아이콘을 누르면 우측에서 사이드 시트가 열려 작업 진행 상황을 볼 수 있어요. 새 로그가 있으면 작은 점 표시가 나타납니다.',
			selector: '[data-tour="log-btn"]',
			interactive: false
		},
		{
			title: '설정',
			body: '톱니 아이콘에서 친구 추가/메시지 전송 후 랜덤 딜레이, 중복 채널 제거, 전송 채널 캐시를 관리할 수 있어요.',
			selector: '[data-tour="settings-btn"]',
			interactive: false
		},
		{
			title: '자동화 실행',
			body: '모든 준비가 끝났다면 이 버튼을 누르세요. 즉시 모드는 모든 채널에, 선택 모드는 체크된 채널에만 메시지를 전송합니다.',
			selector: '[data-tour="primary-btn"]',
			interactive: false
		}
	];
	let tutorialOpen = $state(false);

	function openTutorial() {
		settingsOpen = false;
		tutorialOpen = true;
	}

	// ─── keyword helpers ─────────────────────────────────────────────
	function addKeyword() {
		const v = keywordDraft.trim();
		if (!v || keywords.includes(v)) {
			keywordDraft = '';
			return;
		}
		keywords = [...keywords, v];
		keywordDraft = '';
	}
	function removeKeyword(k: string) {
		keywords = keywords.filter((x) => x !== k);
	}

	// ─── block helpers ───────────────────────────────────────────────
	function addText() {
		blocks = [...blocks, { id: crypto.randomUUID(), kind: 'text', content: '' }];
	}
	function addImage() {
		blocks = [...blocks, { id: crypto.randomUUID(), kind: 'image', path: '' }];
	}
	function removeBlock(id: string) {
		blocks = blocks.filter((b) => b.id !== id);
	}
	function move(id: string, dir: -1 | 1) {
		const i = blocks.findIndex((b) => b.id === id);
		const j = i + dir;
		if (i < 0 || j < 0 || j >= blocks.length) return;
		const next = blocks.slice();
		[next[i], next[j]] = [next[j], next[i]];
		blocks = next;
	}

	// ─── log ─────────────────────────────────────────────────────────
	function pushLog(level: LogPayload['level'], text: string) {
		log = [...log, { id: crypto.randomUUID(), level, text }];
	}

	// ─── extract ─────────────────────────────────────────────────────
	async function onExtract() {
		if (extracting || keywords.length === 0) return;
		extracting = true;
		try {
			const items = await extractChannels(keywords, settings.dedupe);
			extracted = items;
			selectMode = 'manual';
			excludedSet = new Set();
			includedSet = new Set();
		} catch (e) {
			pushLog('err', `추출 실패: ${String(e)}`);
		} finally {
			extracting = false;
		}
	}

	// ─── send ────────────────────────────────────────────────────────
	async function onPrimary() {
		if (running) return;
		if (waitForSelection) {
			await sendSelectedAction();
		} else {
			await runDirectAction();
		}
	}

	async function runDirectAction() {
		running = true;
		try {
			await runAutomation(keywords, blocks, settings.delays);
		} catch (e) {
			pushLog('err', `실행 실패: ${String(e)}`);
		} finally {
			running = false;
		}
	}

	async function sendSelectedAction() {
		const ids = collectSelectedIds();
		if (ids.length === 0) return;
		running = true;
		try {
			await sendToSelected(ids, blocks, settings.delays);
		} catch (e) {
			pushLog('err', `전송 실패: ${String(e)}`);
		} finally {
			running = false;
		}
	}

	async function onCancel() {
		try {
			await cancelWork();
			pushLog('info', '중단 요청을 보냈습니다. 진행 중인 채널 작업이 끝나면 멈춥니다.');
		} catch (e) {
			pushLog('err', `중단 실패: ${String(e)}`);
		}
	}

	function collectSelectedIds(): string[] {
		if (selectMode === 'all') {
			const out: string[] = [];
			for (const c of extracted) {
				if (c.consult && !excludedSet.has(c.id)) out.push(c.id);
			}
			return out;
		}
		return [...includedSet];
	}

	function isSelected(ch: ExtractedChannel): boolean {
		if (!ch.consult) return false;
		return selectMode === 'all' ? !excludedSet.has(ch.id) : includedSet.has(ch.id);
	}

	function toggleOne(ch: ExtractedChannel) {
		if (!ch.consult) return;
		if (selectMode === 'all') {
			const next = new Set(excludedSet);
			if (next.has(ch.id)) next.delete(ch.id);
			else next.add(ch.id);
			excludedSet = next;
		} else {
			const next = new Set(includedSet);
			if (next.has(ch.id)) next.delete(ch.id);
			else next.add(ch.id);
			includedSet = next;
		}
	}

	function toggleAll() {
		if (allSelected) {
			selectMode = 'manual';
			includedSet = new Set();
			excludedSet = new Set();
		} else {
			selectMode = 'all';
			excludedSet = new Set();
			includedSet = new Set();
		}
	}

	function removeExtracted(id: string) {
		extracted = extracted.filter((c) => c.id !== id);
		if (includedSet.has(id)) {
			const next = new Set(includedSet);
			next.delete(id);
			includedSet = next;
		}
		if (excludedSet.has(id)) {
			const next = new Set(excludedSet);
			next.delete(id);
			excludedSet = next;
		}
	}

	function removeSelectedExtracted() {
		const ids = new Set(collectSelectedIds());
		if (ids.size === 0) return;
		extracted = extracted.filter((c) => !ids.has(c.id));
		selectMode = 'manual';
		includedSet = new Set();
		excludedSet = new Set();
	}

	function removeUnselectedExtracted() {
		const ids = new Set(collectSelectedIds());
		if (ids.size === 0) return;
		// keep only selected; reset selection model to all-selected on the survivors
		extracted = extracted.filter((c) => ids.has(c.id));
		selectMode = 'all';
		includedSet = new Set();
		excludedSet = new Set();
	}

	// ─── address filter ──────────────────────────────────────────────
	function addAddressKeyword() {
		const v = addressDraft.trim();
		if (!v || addressKeywords.includes(v)) {
			addressDraft = '';
			return;
		}
		addressKeywords = [...addressKeywords, v];
		addressDraft = '';
	}
	function removeAddressKeyword(k: string) {
		addressKeywords = addressKeywords.filter((x) => x !== k);
	}
	// A keyword may contain whitespace — all whitespace-separated tokens
	// must appear in the address (AND). Multiple keywords are OR'd together.
	// e.g. "부산 동구" matches "부산광역시 동구 ..." but not "서울특별시 동구 ...".
	function matchesAddressKeyword(address: string, keyword: string): boolean {
		if (!address) return false;
		const tokens = keyword.split(/\s+/).filter(Boolean);
		if (tokens.length === 0) return false;
		return tokens.every((t) => address.includes(t));
	}
	function applyAddressFilter() {
		const needles = addressKeywords.map((k) => k.trim()).filter(Boolean);
		if (needles.length === 0) return;
		extracted = extracted.filter((c) =>
			needles.some((n) => matchesAddressKeyword(c.address, n))
		);
		selectMode = 'manual';
		includedSet = new Set();
		excludedSet = new Set();
	}
	let addressFilterPreviewCount = $derived.by(() => {
		const needles = addressKeywords.map((k) => k.trim()).filter(Boolean);
		if (needles.length === 0) return 0;
		return extracted.reduce(
			(n, c) => n + (needles.some((q) => matchesAddressKeyword(c.address, q)) ? 1 : 0),
			0
		);
	});

	// ─── derived ─────────────────────────────────────────────────────
	let chattableCount = $derived(extracted.reduce((n, c) => n + (c.consult ? 1 : 0), 0));
	let selectedCount = $derived(
		selectMode === 'all' ? Math.max(0, chattableCount - excludedSet.size) : includedSet.size
	);
	let allSelected = $derived(
		chattableCount > 0 && selectMode === 'all' && excludedSet.size === 0
	);
	let blocksValid = $derived(
		blocks.length > 0 &&
			blocks.every((b) => (b.kind === 'text' ? b.content.trim() : b.path.trim()))
	);
	let canExtract = $derived(
		!extracting && !running && status === 'running' && waitForSelection && keywords.length > 0
	);
	let canPrimary = $derived(
		!running &&
			!extracting &&
			status === 'running' &&
			blocksValid &&
			(waitForSelection ? selectedCount > 0 : keywords.length > 0)
	);

	// ─── virtualization (fixed-height windowed list) ─────────────────
	const ITEM_H = 72; // px — 3 lines (title / encoded-id+meta / address)
	const VIEWPORT_H = 384; // h-96
	const BUFFER = 6;
	let scrollTop = $state(0);
	let firstIdx = $derived(Math.max(0, Math.floor(scrollTop / ITEM_H) - BUFFER));
	let lastIdx = $derived(
		Math.min(extracted.length, Math.ceil((scrollTop + VIEWPORT_H) / ITEM_H) + BUFFER)
	);
	let visibleSlice = $derived(extracted.slice(firstIdx, lastIdx));

	// ─── theme ───────────────────────────────────────────────────────
	function toggleTheme() {
		theme = theme === 'light' ? 'dark' : 'light';
		document.documentElement.classList.toggle('dark', theme === 'dark');
	}

	async function doCheckKakao() {
		status = 'checking';
		try {
			profile = await checkKakao();
			status = 'running';
			lastError = '';
		} catch (e) {
			console.warn(e);
			lastError = String(e);
			status = 'not_running';
		}
	}

	function fmtCount(n: number) {
		if (n >= 10_000) return `${(n / 10_000).toFixed(1).replace(/\.0$/, '')}만`;
		if (n >= 1_000) return `${(n / 1_000).toFixed(1).replace(/\.0$/, '')}천`;
		return `${n}`;
	}
	function fallback(s: string) {
		return s.replace(/\s+/g, '').slice(0, 1) || '?';
	}

	async function refreshCacheCount() {
		try {
			cacheCount = await getSentCacheCount();
		} catch {
			cacheCount = 0;
		}
	}

	async function onClearCache() {
		await clearSentCache();
		await refreshCacheCount();
	}

	// persist settings whenever they change
	$effect(() => {
		// touch all reactive deps so the effect re-runs on edits
		void settings.delays.addMinSec;
		void settings.delays.addMaxSec;
		void settings.delays.sendMinSec;
		void settings.delays.sendMaxSec;
		void settings.dedupe;
		saveSettings($state.snapshot(settings));
	});

	// when settings dialog opens, refresh the cache count from disk
	$effect(() => {
		if (settingsOpen) refreshCacheCount();
	});

	// ─── lifecycle ───────────────────────────────────────────────────
	onMount(() => {
		settings = loadSettings();
		const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
		theme = prefersDark ? 'dark' : 'light';
		document.documentElement.classList.toggle('dark', prefersDark);
		doCheckKakao();
		refreshCacheCount();

		const unlistenLog = listenLog((line) => pushLog(line.level, line.text));
		const unlistenAuth = listenAuthLost(() => {
			running = false;
			extracting = false;
			status = 'not_running';
		});
		return () => {
			unlistenLog.then((u) => u());
			unlistenAuth.then((u) => u());
		};
	});

	// lock body scroll when forced overlays are visible
	$effect(() => {
		const locked = status === 'not_running' || status === 'checking';
		if (locked) {
			document.body.classList.add('lock-scroll');
			return () => document.body.classList.remove('lock-scroll');
		}
	});
</script>

<div class="min-h-svh bg-background">
	<!-- header -->
	<header class="border-b">
		<div class="mx-auto flex h-14 max-w-3xl items-center gap-4 px-6">
			<h1 class="text-sm font-semibold">KoalaTalk</h1>
			<div data-tour="status-badge">
				{#if status === 'running'}
					<Badge variant="secondary" class="gap-1.5">
						<span class="size-1.5 rounded-full bg-emerald-500"></span>
						연결됨
					</Badge>
				{:else if status === 'checking'}
					<Badge variant="secondary" class="gap-1.5">
						<Loader2 class="size-3 animate-spin" />
						확인 중
					</Badge>
				{:else}
					<Badge variant="destructive">미실행</Badge>
				{/if}
			</div>

			<div class="ml-auto flex items-center gap-2">
				{#if status === 'running' && profile}
					<div class="flex items-center gap-2" data-tour="profile">
						<Avatar
							src={profile.profileUrl}
							alt={profile.name}
							fallback={fallback(profile.name)}
							size={28}
						/>
						<span class="hidden text-xs font-medium sm:inline">{profile.name}</span>
					</div>
				{/if}
				<Button
					variant="ghost"
					size="icon"
					onclick={() => (logSheetOpen = true)}
					class="relative"
					title="실행 로그"
					data-tour="log-btn"
				>
					<Terminal class="size-4" />
					{#if log.length > 0}
						<span
							class="absolute right-1.5 top-1.5 size-1.5 rounded-full bg-primary"
						></span>
					{/if}
				</Button>
				<Button
					variant="ghost"
					size="icon"
					onclick={() => (settingsOpen = true)}
					title="설정"
					data-tour="settings-btn"
				>
					<Settings class="size-4" />
				</Button>
				<Button variant="ghost" size="icon" onclick={toggleTheme} title="테마">
					{#if theme === 'dark'}
						<Sun class="size-4" />
					{:else}
						<Moon class="size-4" />
					{/if}
				</Button>
			</div>
		</div>
	</header>

	<!-- body -->
	<main class="mx-auto max-w-3xl px-6 py-8">
		<div class="mb-6">
			<h2 class="text-xl font-semibold tracking-tight">자동화 설정</h2>
			<p class="mt-1 text-sm text-muted-foreground">
				키워드로 카카오톡 채널을 검색해, 1:1 문의 가능한 채널에 메시지 블럭을 전송합니다.
			</p>
		</div>

		<div class="flex flex-col gap-6">
			<!-- combined: keywords + extracted channels -->
			<Card>
				<div class="flex items-start gap-3 p-6 pb-4">
					<div data-tour="wait-toggle" class="mt-0.5">
						<Checkbox bind:checked={waitForSelection} />
					</div>
					<button
						type="button"
						onclick={() => (waitForSelection = !waitForSelection)}
						class="flex-1 text-left"
					>
						<h3 class="text-sm font-semibold leading-none">검색 키워드</h3>
						<p class="mt-1.5 text-xs text-muted-foreground">
							{#if waitForSelection}
								추출 후 채널 선택 대기 — "추출" 버튼으로 검색만 수행한 뒤 채널을 선택하세요.
							{:else}
								즉시 모드 — 검색된 모든 1:1 가능 채널에 자동 전송됩니다.
							{/if}
						</p>
					</button>
					<Button
						variant="outline"
						size="sm"
						onclick={onExtract}
						disabled={!canExtract}
						title={canExtract ? '채널 추출' : '"추출 후 채널 선택 대기" 를 켜세요'}
						data-tour="extract-btn"
					>
						{#if extracting}
							<Loader2 class="size-3.5 animate-spin" />
						{:else}
							<Search class="size-3.5" />
						{/if}
						추출
					</Button>
				</div>

				<div class="px-6 pb-6">
					<div
						data-tour="keyword-input"
						class="flex flex-wrap items-center gap-1.5 rounded-md border border-input bg-transparent p-2 transition-[border-color,box-shadow] focus-within:border-ring focus-within:ring-2 focus-within:ring-ring/30"
					>
						{#each keywords as k (k)}
							<span
								class="inline-flex items-center gap-1 rounded-md bg-secondary py-1 pl-2 pr-1 text-xs text-secondary-foreground"
							>
								{k}
								<button
									type="button"
									onclick={() => removeKeyword(k)}
									class="flex size-4 items-center justify-center rounded text-muted-foreground hover:bg-foreground/10 hover:text-foreground"
									aria-label="{k} 제거"
								>
									<X class="size-3" />
								</button>
							</span>
						{/each}
						<input
							bind:value={keywordDraft}
							onkeydown={(e) => {
								if (e.key === 'Enter') {
									e.preventDefault();
									addKeyword();
								} else if (e.key === 'Backspace' && !keywordDraft && keywords.length) {
									keywords = keywords.slice(0, -1);
								}
							}}
							placeholder={keywords.length === 0 ? '키워드 입력 후 Enter' : '추가...'}
							class="min-w-[120px] flex-1 bg-transparent px-1.5 py-0.5 text-sm outline-none placeholder:text-muted-foreground/70"
						/>
					</div>
				</div>

				{#if waitForSelection}
					<div class="border-t">
						<div class="flex items-center justify-between gap-3 px-6 py-3">
							<div class="flex items-center gap-2">
								<Checkbox
									checked={allSelected}
									indeterminate={selectedCount > 0 && !allSelected}
									onCheckedChange={() => toggleAll()}
									disabled={extracted.length === 0}
								/>
								<span class="text-xs text-muted-foreground">
									추출된 채널
									{#if extracted.length > 0}
										· 총 {fmtCount(extracted.length)}개 · 1:1 가능 {fmtCount(chattableCount)}개 · 선택 {fmtCount(selectedCount)}개
									{:else}
										· 추출 결과 없음
									{/if}
								</span>
							</div>
							{#if selectedCount > 0}
								<div class="flex gap-1.5">
									<button
										type="button"
										onclick={removeSelectedExtracted}
										class="inline-flex items-center gap-1 rounded-md border border-destructive/30 bg-destructive/5 px-2 py-1 text-[11px] font-medium text-destructive transition-colors hover:bg-destructive/10"
									>
										<Trash2 class="size-3" />
										선택 삭제 ({fmtCount(selectedCount)})
									</button>
									<button
										type="button"
										onclick={removeUnselectedExtracted}
										class="inline-flex items-center gap-1 rounded-md border bg-muted/40 px-2 py-1 text-[11px] font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
									>
										<Trash2 class="size-3" />
										미선택 삭제 ({fmtCount(extracted.length - selectedCount)})
									</button>
								</div>
							{/if}
						</div>

						{#if extracted.length > 0}
							<div class="border-t px-6 py-3">
								<div class="mb-2 flex items-center justify-between gap-2">
									<span class="text-[11px] font-medium text-muted-foreground">
										주소 필터 — 키워드가 주소에 포함된 채널만 남깁니다 (한 키워드 안 공백은 AND, 예: "부산 동구")
									</span>
									<button
										type="button"
										onclick={applyAddressFilter}
										disabled={addressKeywords.length === 0}
										class="inline-flex items-center gap-1 rounded-md border border-primary/30 bg-primary/5 px-2 py-1 text-[11px] font-medium text-primary transition-colors enabled:hover:bg-primary/10 disabled:opacity-40"
									>
										<Trash2 class="size-3" />
										정리
										{#if addressKeywords.length > 0}
											({fmtCount(addressFilterPreviewCount)} 유지 · {fmtCount(
												extracted.length - addressFilterPreviewCount
											)} 제거)
										{/if}
									</button>
								</div>
								<div
									class="flex flex-wrap items-center gap-1.5 rounded-md border border-input bg-transparent p-2 transition-[border-color,box-shadow] focus-within:border-ring focus-within:ring-2 focus-within:ring-ring/30"
								>
									{#each addressKeywords as k (k)}
										<span
											class="inline-flex items-center gap-1 rounded-md bg-secondary py-1 pl-2 pr-1 text-xs text-secondary-foreground"
										>
											{k}
											<button
												type="button"
												onclick={() => removeAddressKeyword(k)}
												class="flex size-4 items-center justify-center rounded text-muted-foreground hover:bg-foreground/10 hover:text-foreground"
												aria-label="{k} 제거"
											>
												<X class="size-3" />
											</button>
										</span>
									{/each}
									<input
										bind:value={addressDraft}
										onkeydown={(e) => {
											if (e.key === 'Enter') {
												e.preventDefault();
												addAddressKeyword();
											} else if (
												e.key === 'Backspace' &&
												!addressDraft &&
												addressKeywords.length
											) {
												addressKeywords = addressKeywords.slice(0, -1);
											}
										}}
										placeholder={addressKeywords.length === 0
											? '예: 서울, 강남구, 부산 동구 — Enter 로 추가 (공백 = AND)'
											: '추가...'}
										class="min-w-[120px] flex-1 bg-transparent px-1.5 py-0.5 text-xs outline-none placeholder:text-muted-foreground/70"
									/>
								</div>
							</div>
							<div
								onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
								class="relative h-96 overflow-y-auto border-t"
							>
								<div
									class="relative"
									style="height: {extracted.length * ITEM_H}px;"
								>
									{#each visibleSlice as ch, i (ch.id)}
										{@const idx = firstIdx + i}
										<div
											style="position:absolute; top:{idx * ITEM_H}px; left:0; right:0; height:{ITEM_H}px;"
											class={cn(
												'group flex items-center gap-3 px-6 transition-colors',
												ch.consult ? 'hover:bg-muted/40' : 'opacity-50 hover:opacity-80'
											)}
										>
											<Checkbox
												checked={isSelected(ch)}
												disabled={!ch.consult}
												onCheckedChange={() => toggleOne(ch)}
											/>
											<Avatar
												src={ch.imageUrl}
												alt={ch.title}
												fallback={fallback(ch.title)}
												size={32}
											/>
											<div class="min-w-0 flex-1">
												<div class="flex items-center gap-2">
													<span class="truncate text-sm font-medium">{ch.title}</span>
													{#if !ch.consult}
														<Badge variant="outline">1:1 미지원</Badge>
													{/if}
												</div>
												<p class="truncate text-[11px] text-muted-foreground">
													{ch.encodedId} · 친구 {fmtCount(ch.friendCount)} · #{ch.keyword}
												</p>
												<p class="truncate text-[11px] text-muted-foreground/80">
													{ch.address || '주소 없음'}
												</p>
											</div>
											<button
												type="button"
												onclick={() => removeExtracted(ch.id)}
												class="flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground opacity-0 transition-opacity hover:bg-destructive/10 hover:text-destructive group-hover:opacity-100 focus-visible:opacity-100"
												aria-label="채널 삭제"
												title="목록에서 삭제"
											>
												<X class="size-3.5" />
											</button>
										</div>
									{/each}
								</div>
							</div>
						{/if}
					</div>
				{/if}
			</Card>

			<!-- blocks -->
			<div data-tour="blocks-card">
				<Card>
					<div class="flex items-end justify-between gap-3 p-6 pb-4">
					<div class="flex flex-col gap-1.5">
						<h3 class="text-sm font-semibold leading-none">전송 메시지</h3>
						<p class="text-xs text-muted-foreground">블럭 순서대로 채팅창에 순차 전송됩니다.</p>
					</div>
					<div class="flex gap-2">
						<Button variant="outline" size="sm" onclick={addText}>
							<Plus class="size-3.5" />
							텍스트
						</Button>
						<Button variant="outline" size="sm" onclick={addImage}>
							<Plus class="size-3.5" />
							이미지
						</Button>
					</div>
				</div>

				<div class="flex flex-col gap-3 px-6 pb-6">
					{#each blocks as b, i (b.id)}
						<div class="flex gap-3 rounded-md border p-3">
							<div class="flex flex-col items-center gap-1 pt-1">
								<span class="text-xs font-medium tabular-nums text-muted-foreground">
									{i + 1}
								</span>
								{#if b.kind === 'text'}
									<Type class="size-3.5 text-muted-foreground" />
								{:else}
									<ImageIcon class="size-3.5 text-muted-foreground" />
								{/if}
							</div>

							<div class="min-w-0 flex-1">
								{#if b.kind === 'text'}
									<Textarea bind:value={b.content} placeholder="전송할 텍스트" rows={2} />
								{:else}
									<div class="flex gap-2">
										<Input bind:value={b.path} placeholder="C:\path\to\image.png" />
										<Button
											variant="outline"
											onclick={async () => {
												const picked = await pickImageFile();
												if (picked) b.path = picked;
											}}
										>
											파일 선택
										</Button>
									</div>
								{/if}
							</div>

							<div class="flex flex-col gap-1">
								<button
									type="button"
									onclick={() => move(b.id, -1)}
									disabled={i === 0}
									class="flex size-7 items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-accent-foreground disabled:opacity-30"
									aria-label="위로"
								>
									<ChevronUp class="size-3.5" />
								</button>
								<button
									type="button"
									onclick={() => move(b.id, 1)}
									disabled={i === blocks.length - 1}
									class="flex size-7 items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-accent-foreground disabled:opacity-30"
									aria-label="아래로"
								>
									<ChevronDown class="size-3.5" />
								</button>
								<button
									type="button"
									onclick={() => removeBlock(b.id)}
									class="flex size-7 items-center justify-center rounded-md text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
									aria-label="삭제"
								>
									<Trash2 class="size-3.5" />
								</button>
							</div>
						</div>
					{/each}

					{#if blocks.length === 0}
						<p class="py-6 text-center text-xs text-muted-foreground">추가된 블럭이 없습니다.</p>
					{/if}
				</div>
			</Card>
			</div>

			<!-- fixed-position primary action -->
			<div class="flex items-center justify-between gap-4">
				<p class="text-xs text-muted-foreground">
					{#if waitForSelection}
						선택 모드: 추출된 채널 중 {fmtCount(selectedCount)}개 선택
					{:else}
						즉시 모드: {keywords.length} 키워드 × {blocks.length} 블럭
					{/if}
				</p>
				<div class="flex gap-2">
					{#if running || extracting}
						<Button variant="outline" onclick={onCancel}>
							<Square class="size-4" />
							중단
						</Button>
					{/if}
					<Button onclick={onPrimary} disabled={!canPrimary} data-tour="primary-btn">
						{#if extracting}
							<Loader2 class="size-4 animate-spin" />
							추출 중
						{:else if running}
							<Loader2 class="size-4 animate-spin" />
							실행 중
						{:else if waitForSelection}
							<Send class="size-4" />
							선택 채널에 전송 ({fmtCount(selectedCount)})
						{:else}
							<Play class="size-4" />
							자동화 실행
						{/if}
					</Button>
				</div>
			</div>
		</div>
	</main>
</div>

<!-- log side sheet -->
<Sheet bind:open={logSheetOpen} title="실행 로그">
	{#snippet actions()}
		{#if log.length > 0}
			<button
				type="button"
				onclick={() => (log = [])}
				class="rounded-md px-2 py-1 text-xs text-muted-foreground hover:bg-accent hover:text-accent-foreground"
			>
				비우기
			</button>
		{/if}
	{/snippet}
	{#snippet footer()}
		<div class="flex items-center justify-start">
			<button
				type="button"
				onclick={async () => {
					const { openUrl } = await import('@tauri-apps/plugin-opener');
					await openUrl('https://app-solution.co.kr/');
				}}
				class="text-[11px] text-muted-foreground underline-offset-4 transition-colors hover:text-foreground hover:underline"
			>
				App Solution
			</button>
		</div>
	{/snippet}
	{#if log.length === 0}
		<div class="flex h-full flex-col items-center justify-center gap-2 px-6 text-center">
			<div class="flex size-12 items-center justify-center rounded-full bg-muted text-muted-foreground">
				<Terminal class="size-5" />
			</div>
			<p class="text-xs text-muted-foreground">아직 실행 기록이 없습니다.</p>
		</div>
	{:else}
		<ul class="flex flex-col px-5 py-4 font-mono text-xs leading-6">
			{#each log as line (line.id)}
				<li
					class={cn(
						'flex gap-2',
						line.level === 'ok' && 'text-foreground',
						line.level === 'skip' && 'text-muted-foreground',
						line.level === 'err' && 'text-destructive',
						line.level === 'info' && 'text-muted-foreground'
					)}
				>
					<span class="select-none">
						{#if line.level === 'ok'}
							[OK]
						{:else if line.level === 'skip'}
							[SKIP]
						{:else if line.level === 'err'}
							[ERR]
						{:else}
							[..]
						{/if}
					</span>
					<span class="break-words">{line.text}</span>
				</li>
			{/each}
		</ul>
	{/if}
</Sheet>

<!-- not running modal (forced, non-dismissible) -->
{#if status === 'not_running'}
	<div class="fixed inset-0 z-50 flex items-center justify-center p-4">
		<div
			class="absolute inset-0 bg-black/50 backdrop-blur-[2px]"
			transition:fade={{ duration: 160, easing: cubicOut }}
		></div>
		<div
			class="relative z-10 w-full max-w-sm rounded-lg border bg-card shadow-xl"
			transition:scale={{ duration: 180, start: 0.96, opacity: 0, easing: cubicOut }}
		>
			<div class="flex flex-col gap-1.5 p-6 pb-4">
				<h2 class="flex items-center gap-2 text-base font-semibold">
					<AlertTriangle class="size-4 text-destructive" />
					카카오톡 연결 실패
				</h2>
				<p class="text-sm text-muted-foreground">
					KoalaTalk 은 카카오톡 PC가 실행 중이고 로그인된 상태에서만 동작합니다. 카카오톡 실행 →
					로그인 후 다시 시도해 주세요.
				</p>
				{#if lastError}
					<div class="mt-2 rounded-md border border-destructive/30 bg-destructive/5 p-2.5">
						<p class="font-mono text-[11px] leading-relaxed text-destructive break-words">
							{lastError}
						</p>
					</div>
				{/if}
			</div>
			<div class="flex items-center justify-between gap-2 px-6 pb-6">
				<button
					type="button"
					onclick={() => (logSheetOpen = true)}
					class="text-xs text-muted-foreground underline-offset-4 hover:text-foreground hover:underline"
				>
					로그 보기
				</button>
				<div class="flex gap-2">
					<Button variant="outline" onclick={doCheckKakao}>
						<RefreshCw class="size-3.5" />
						다시 확인
					</Button>
					<Button variant="destructive" onclick={exitApp}>
						<LogOut class="size-3.5" />
						종료
					</Button>
				</div>
			</div>
		</div>
	</div>
{/if}

<!-- checking overlay -->
{#if status === 'checking'}
	<div
		class="fixed inset-0 z-40 flex items-center justify-center bg-background/80 backdrop-blur-[2px]"
		transition:fade={{ duration: 160, easing: cubicOut }}
	>
		<div class="flex items-center gap-2 text-sm text-muted-foreground">
			<Loader2 class="size-4 animate-spin" />
			카카오톡 확인 중...
		</div>
	</div>
{/if}

<!-- settings dialog -->
<Dialog bind:open={settingsOpen}>
	<div class="flex flex-col gap-1.5">
		<h2 class="text-base font-semibold leading-none">설정</h2>
		<p class="text-sm text-muted-foreground">전송 동작 옵션을 지정합니다.</p>
	</div>

	<div class="flex flex-col gap-5 py-2">
		<div class="grid gap-2">
			<label for="add-min" class="text-sm font-medium">친구 추가 후 딜레이 (초)</label>
			<div class="flex items-center gap-2">
				<Input
					id="add-min"
					type="number"
					min={0}
					step={0.1}
					bind:value={settings.delays.addMinSec}
				/>
				<span class="text-sm text-muted-foreground">~</span>
				<Input type="number" min={0} step={0.1} bind:value={settings.delays.addMaxSec} />
			</div>
			<p class="text-[11px] text-muted-foreground">
				친구 추가 후 메시지를 보내기 전까지 n~m 초 사이 랜덤 대기.
			</p>
		</div>

		<div class="grid gap-2">
			<label for="send-min" class="text-sm font-medium">메시지 전송 후 딜레이 (초)</label>
			<div class="flex items-center gap-2">
				<Input
					id="send-min"
					type="number"
					min={0}
					step={0.1}
					bind:value={settings.delays.sendMinSec}
				/>
				<span class="text-sm text-muted-foreground">~</span>
				<Input type="number" min={0} step={0.1} bind:value={settings.delays.sendMaxSec} />
			</div>
			<p class="text-[11px] text-muted-foreground">
				한 채널 작업 완료 후 다음 채널로 넘어가기 전 대기.
			</p>
		</div>

		<div class="flex items-start gap-3 rounded-md border p-3">
			<Checkbox bind:checked={settings.dedupe} class="mt-0.5" />
			<button
				type="button"
				onclick={() => (settings.dedupe = !settings.dedupe)}
				class="flex-1 text-left"
			>
				<p class="text-sm font-medium">다른 키워드 같은 채널 중복 제거</p>
				<p class="mt-0.5 text-[11px] text-muted-foreground">
					여러 키워드에서 동일 채널이 검색된 경우 첫 키워드 결과만 유지.
				</p>
			</button>
		</div>

		<div class="flex items-center justify-between gap-3 rounded-md border p-3">
			<div>
				<p class="text-sm font-medium">전송 채널 캐시</p>
				<p class="mt-0.5 text-[11px] text-muted-foreground">
					이미 메시지를 보낸 채널 {cacheCount}개. 다음 실행에서 자동으로 건너뜁니다.
				</p>
			</div>
			<Button
				variant="outline"
				size="sm"
				onclick={onClearCache}
				disabled={cacheCount === 0}
			>
				<Trash2 class="size-3.5" />
				캐시 삭제
			</Button>
		</div>
	</div>

	<div class="flex items-center justify-between gap-2">
		<button
			type="button"
			onclick={openTutorial}
			class="text-xs text-muted-foreground underline-offset-4 transition-colors hover:text-foreground hover:underline"
		>
			어떻게 사용 하나요?
		</button>
		<div class="flex gap-2">
			<Button
				variant="outline"
				onclick={() => {
					settings = { delays: { ...defaultSettings.delays }, dedupe: defaultSettings.dedupe };
				}}
			>
				기본값
			</Button>
			<Button onclick={() => (settingsOpen = false)}>완료</Button>
		</div>
	</div>
</Dialog>

<!-- interactive tutorial overlay (demo) -->
<Tutorial bind:open={tutorialOpen} steps={tutorialSteps} />
