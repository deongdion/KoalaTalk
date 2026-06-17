export type Profile = {
	name: string;
	email: string;
	profileUrl: string;
};

export type TextBlock = { id: string; kind: 'text'; content: string };
export type ImageBlock = { id: string; kind: 'image'; path: string };
export type MessageBlock = TextBlock | ImageBlock;

export type KakaoStatus = 'checking' | 'running' | 'not_running';

export type ExtractedChannel = {
	id: string;
	title: string;
	encodedId: string;
	imageUrl: string;
	friendCount: number;
	consult: boolean;
	keyword: string;
	address: string;
};

export type Delays = {
	addMinSec: number;
	addMaxSec: number;
	sendMinSec: number;
	sendMaxSec: number;
	windowOpenSec: number;
	pasteDelaySec: number;
};

export type Settings = {
	delays: Delays;
	dedupe: boolean;
};

export const defaultDelays: Delays = {
	addMinSec: 1.5,
	addMaxSec: 3,
	sendMinSec: 0.5,
	sendMaxSec: 1.5,
	windowOpenSec: 2.5,
	pasteDelaySec: 0.5
};

export const defaultSettings: Settings = {
	delays: defaultDelays,
	dedupe: true
};

const STORAGE_KEY = 'koalatalk:settings';

export function loadSettings(): Settings {
	if (typeof localStorage === 'undefined') return structuredClone(defaultSettings);
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return structuredClone(defaultSettings);
		const parsed = JSON.parse(raw);
		return {
			delays: { ...defaultDelays, ...(parsed.delays ?? {}) },
			dedupe: parsed.dedupe ?? defaultSettings.dedupe
		};
	} catch {
		return structuredClone(defaultSettings);
	}
}

export function saveSettings(s: Settings) {
	if (typeof localStorage === 'undefined') return;
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
	} catch {
		/* ignore */
	}
}
