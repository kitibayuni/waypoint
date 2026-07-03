import { writable } from 'svelte/store';

/** Shared "as of" timestamp driving both the graph pane and the feed pane on the replay view. */
export const asOfTimestamp = writable<string | null>(null);
