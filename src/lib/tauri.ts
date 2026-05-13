import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Delays, ExtractedChannel, MessageBlock, Profile } from './data';

export const isTauri =
	typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

type BlockPayload =
	| { kind: 'text'; content: string }
	| { kind: 'image'; path: string };

function toPayload(blocks: MessageBlock[]): BlockPayload[] {
	return blocks.map((b) =>
		b.kind === 'text' ? { kind: 'text', content: b.content } : { kind: 'image', path: b.path }
	);
}

export async function checkKakao(): Promise<Profile> {
	if (!isTauri) {
		// Browser preview cannot extract auth token from KakaoTalk.exe memory.
		throw new Error('not_running');
	}
	return await tauriInvoke<Profile>('check_kakao');
}

export async function extractChannels(
	keywords: string[],
	dedupe: boolean
): Promise<ExtractedChannel[]> {
	if (!isTauri) {
		await new Promise((r) => setTimeout(r, 600));
		const items = keywords.flatMap((kw, ki) =>
			Array.from({ length: 4 }, (_, i) => ({
				id: `ch-${i}`, // intentionally same ids across keywords to test dedupe
				title: ['브이커피', '서울베이글', '한강브런치', '판교라멘'][i],
				encodedId: `@ch-${i}`,
				imageUrl: '',
				friendCount: Math.floor(Math.random() * 50_000),
				consult: Math.random() > 0.3,
				keyword: kw
			}))
		);
		if (!dedupe) return items;
		const seen = new Set<string>();
		return items.filter((c) => (seen.has(c.id) ? false : (seen.add(c.id), true)));
	}
	return await tauriInvoke<ExtractedChannel[]>('extract_channels', { keywords, dedupe });
}

export async function runAutomation(
	keywords: string[],
	blocks: MessageBlock[],
	delays: Delays
): Promise<{ ok: boolean; sent: number; skipped: number }> {
	if (!isTauri) {
		await new Promise((r) => setTimeout(r, 1500));
		return { ok: true, sent: keywords.length * 2, skipped: 1 };
	}
	return await tauriInvoke('run_automation', { keywords, blocks: toPayload(blocks), delays });
}

export async function sendToSelected(
	channelIds: string[],
	blocks: MessageBlock[],
	delays: Delays
): Promise<{ ok: boolean; sent: number; skipped: number }> {
	if (!isTauri) {
		await new Promise((r) => setTimeout(r, 1500));
		return { ok: true, sent: channelIds.length, skipped: 0 };
	}
	return await tauriInvoke('send_to_selected', {
		channelIds,
		blocks: toPayload(blocks),
		delays
	});
}

export type LogPayload = { level: 'info' | 'ok' | 'skip' | 'err'; text: string };

export async function listenLog(
	cb: (line: LogPayload) => void
): Promise<UnlistenFn> {
	if (!isTauri) {
		return () => {};
	}
	return await tauriListen<LogPayload>('log', (e) => cb(e.payload));
}

export async function listenAuthLost(cb: () => void): Promise<UnlistenFn> {
	if (!isTauri) return () => {};
	return await tauriListen('auth_lost', () => cb());
}

export async function getSentCacheCount(): Promise<number> {
	if (!isTauri) return 0;
	return await tauriInvoke<number>('get_sent_cache_count');
}

export async function clearSentCache(): Promise<void> {
	if (!isTauri) return;
	await tauriInvoke('clear_sent_cache');
}

export async function cancelWork(): Promise<void> {
	if (!isTauri) return;
	await tauriInvoke('cancel_work');
}

export async function pickImageFile(): Promise<string | null> {
	if (!isTauri) return null;
	const { open } = await import('@tauri-apps/plugin-dialog');
	const picked = await open({
		multiple: false,
		directory: false,
		filters: [{ name: 'Image', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp'] }]
	});
	if (typeof picked === 'string') return picked;
	if (Array.isArray(picked) && picked.length > 0) return picked[0];
	return null;
}

export async function exitApp() {
	if (!isTauri) {
		alert('실제 환경에서는 프로그램이 종료됩니다.');
		return;
	}
	const { getCurrentWindow } = await import('@tauri-apps/api/window');
	await getCurrentWindow().close();
}
