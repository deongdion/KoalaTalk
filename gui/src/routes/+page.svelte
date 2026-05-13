<script lang="ts">
	import { onMount } from 'svelte';
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
		Trash2
	} from '@lucide/svelte';

	import Button from '$lib/components/ui/button.svelte';
	import Input from '$lib/components/ui/input.svelte';
	import Textarea from '$lib/components/ui/textarea.svelte';
	import Avatar from '$lib/components/ui/avatar.svelte';
	import Badge from '$lib/components/ui/badge.svelte';
	import Card from '$lib/components/ui/card.svelte';
	import { mockProfile, type KakaoStatus, type MessageBlock } from '$lib/data';
	import { cn } from '$lib/utils';

	// ─── status ───────────────────────────────────────────────────────
	let status = $state<KakaoStatus>('checking');
	let profile = $state(mockProfile);
	let theme = $state<'light' | 'dark'>('light');

	// ─── keywords ────────────────────────────────────────────────────
	let keywords = $state<string[]>(['카페', '맛집']);
	let keywordDraft = $state('');

	function addKeyword() {
		const v = keywordDraft.trim();
		if (!v) return;
		if (keywords.includes(v)) {
			keywordDraft = '';
			return;
		}
		keywords = [...keywords, v];
		keywordDraft = '';
	}
	function removeKeyword(k: string) {
		keywords = keywords.filter((x) => x !== k);
	}

	// ─── message blocks ──────────────────────────────────────────────
	let blocks = $state<MessageBlock[]>([
		{
			id: crypto.randomUUID(),
			kind: 'text',
			content: '안녕하세요! KoalaTalk 입니다.'
		}
	]);

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

	// ─── run ─────────────────────────────────────────────────────────
	type LogLine = { id: string; level: 'info' | 'ok' | 'skip' | 'err'; text: string };
	let running = $state(false);
	let log = $state<LogLine[]>([]);

	function pushLog(level: LogLine['level'], text: string) {
		log = [...log, { id: crypto.randomUUID(), level, text }];
	}

	async function run() {
		if (running) return;
		running = true;
		log = [];
		pushLog('info', `키워드 ${keywords.length}개 · 블럭 ${blocks.length}개로 시작.`);

		for (const kw of keywords) {
			pushLog('info', `"${kw}" 검색 중...`);
			await new Promise((r) => setTimeout(r, 400));
			const found = Math.floor(8 + Math.random() * 12);
			pushLog('info', `${found}개 채널 발견.`);

			for (let i = 0; i < Math.min(3, found); i++) {
				const consult = Math.random() > 0.35;
				const name = ['브이커피', '서울베이글', '한강브런치', '판교라멘'][i % 4] + (i + 1);
				if (!consult) {
					pushLog('skip', `${name}: 1:1 문의 미지원, 건너뜀.`);
					continue;
				}
				await new Promise((r) => setTimeout(r, 200));
				pushLog('ok', `${name}: 친구추가 후 메시지 ${blocks.length}개 전송.`);
			}
		}
		pushLog('ok', '모든 작업이 완료되었습니다.');
		running = false;
	}

	// ─── derived ─────────────────────────────────────────────────────
	let totalActions = $derived(keywords.length * blocks.length);
	let canRun = $derived(
		!running &&
			status === 'running' &&
			keywords.length > 0 &&
			blocks.length > 0 &&
			blocks.every((b) => (b.kind === 'text' ? b.content.trim() : b.path.trim()))
	);

	// ─── theme ───────────────────────────────────────────────────────
	function toggleTheme() {
		theme = theme === 'light' ? 'dark' : 'light';
		document.documentElement.classList.toggle('dark', theme === 'dark');
	}

	// ─── kakao check ─────────────────────────────────────────────────
	async function checkKakao() {
		status = 'checking';
		await new Promise((r) => setTimeout(r, 500));
		status = 'running';
	}

	function exitApp() {
		alert('실제 환경에서는 프로그램이 종료됩니다.');
	}

	onMount(() => {
		const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
		theme = prefersDark ? 'dark' : 'light';
		document.documentElement.classList.toggle('dark', prefersDark);
		checkKakao();
	});
</script>

<div class="min-h-svh bg-background">
	<!-- header -->
	<header class="border-b">
		<div class="mx-auto flex h-14 max-w-3xl items-center gap-4 px-6">
			<h1 class="text-sm font-semibold">KoalaTalk</h1>
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

			<div class="ml-auto flex items-center gap-2">
				{#if status === 'running'}
					<div class="flex items-center gap-2">
						<Avatar
							src={profile.profileUrl}
							alt={profile.name}
							fallback={profile.name.slice(0, 1)}
							size={28}
						/>
						<div class="hidden flex-col leading-tight sm:flex">
							<span class="text-xs font-medium">{profile.name}</span>
							<span class="text-[11px] text-muted-foreground">{profile.email}</span>
						</div>
					</div>
				{/if}
				<Button variant="ghost" size="icon" onclick={toggleTheme}>
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
				키워드로 카카오톡 채널을 검색해, 1:1 문의 가능한 채널에 메시지 블럭을 차례로 전송합니다.
			</p>
		</div>

		<div class="flex flex-col gap-6">
			<!-- keywords -->
			<Card>
				<div class="flex flex-col gap-1.5 p-6 pb-4">
					<h3 class="text-sm font-semibold leading-none">검색 키워드</h3>
					<p class="text-xs text-muted-foreground">
						Enter 로 추가 · Backspace 로 마지막 키워드 제거
					</p>
				</div>
				<div class="px-6 pb-6">
					<div
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
			</Card>

			<!-- blocks -->
			<Card>
				<div class="flex items-end justify-between gap-3 p-6 pb-4">
					<div class="flex flex-col gap-1.5">
						<h3 class="text-sm font-semibold leading-none">전송 메시지</h3>
						<p class="text-xs text-muted-foreground">
							블럭 순서대로 채팅창에 순차 전송됩니다.
						</p>
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
								<span class="text-xs font-medium text-muted-foreground tabular-nums">
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
									<Textarea
										bind:value={b.content}
										placeholder="전송할 텍스트"
										rows={2}
									/>
								{:else}
									<div class="flex gap-2">
										<Input bind:value={b.path} placeholder="C:\path\to\image.png" />
										<Button variant="outline" size="default">파일 선택</Button>
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
						<p class="py-6 text-center text-xs text-muted-foreground">
							추가된 블럭이 없습니다. 텍스트나 이미지를 추가하세요.
						</p>
					{/if}
				</div>
			</Card>

			<!-- actions -->
			<div class="flex items-center justify-between gap-4">
				<p class="text-xs text-muted-foreground">
					{keywords.length} 키워드 × {blocks.length} 블럭 = 채널당 최대
					<span class="font-medium text-foreground">{blocks.length}</span> 회 전송
				</p>
				<Button onclick={run} disabled={!canRun}>
					{#if running}
						<Loader2 class="size-4 animate-spin" />
						실행 중
					{:else}
						<Play class="size-4" />
						자동화 실행
					{/if}
				</Button>
			</div>

			<!-- log -->
			{#if log.length > 0}
				<Card>
					<div class="flex items-center justify-between gap-3 p-6 pb-3">
						<h3 class="text-sm font-semibold leading-none">실행 로그</h3>
						<button
							onclick={() => (log = [])}
							class="text-xs text-muted-foreground hover:text-foreground"
						>
							비우기
						</button>
					</div>
					<div class="max-h-72 overflow-y-auto px-6 pb-6">
						<ul class="flex flex-col font-mono text-xs leading-6">
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
					</div>
				</Card>
			{/if}
		</div>
	</main>
</div>

<!-- not running modal -->
{#if status === 'not_running'}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
	>
		<Card class="w-full max-w-sm">
			<div class="flex flex-col gap-1.5 p-6 pb-4">
				<h2 class="flex items-center gap-2 text-base font-semibold">
					<AlertTriangle class="size-4 text-destructive" />
					카카오톡 미실행
				</h2>
				<p class="text-sm text-muted-foreground">
					KoalaTalk 은 카카오톡 PC가 실행 중이어야 동작합니다. 카카오톡을 먼저 실행한 뒤 다시 시도해 주세요.
				</p>
			</div>
			<div class="flex justify-end gap-2 px-6 pb-6">
				<Button variant="outline" onclick={checkKakao}>
					<RefreshCw class="size-3.5" />
					다시 확인
				</Button>
				<Button variant="destructive" onclick={exitApp}>
					<LogOut class="size-3.5" />
					종료
				</Button>
			</div>
		</Card>
	</div>
{/if}

<!-- checking overlay -->
{#if status === 'checking'}
	<div
		class="fixed inset-0 z-40 flex items-center justify-center bg-background/80 backdrop-blur-sm"
	>
		<div class="flex items-center gap-2 text-sm text-muted-foreground">
			<Loader2 class="size-4 animate-spin" />
			카카오톡 확인 중...
		</div>
	</div>
{/if}
