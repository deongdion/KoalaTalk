export type Profile = {
	name: string;
	email: string;
	profileUrl: string;
};

export type TextBlock = { id: string; kind: 'text'; content: string };
export type ImageBlock = { id: string; kind: 'image'; path: string; sizeMb?: number };
export type MessageBlock = TextBlock | ImageBlock;

export type KakaoStatus = 'checking' | 'running' | 'not_running';

export const mockProfile: Profile = {
	name: '이민기',
	email: 'leeminki78885@gmail.com',
	profileUrl: 'https://api.dicebear.com/9.x/notionists/svg?seed=minki&backgroundColor=fee500'
};
