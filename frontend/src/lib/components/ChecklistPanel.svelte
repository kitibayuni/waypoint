<script lang="ts">
	import type { Checklist } from '$lib/api/checklists';
	import { updateChecklistItemState } from '$lib/api/checklists';

	type ChecklistItemState = 'todo' | 'doing' | 'done' | 'na';

	let { checklist, onchange }: { checklist: Checklist; onchange?: (checklist: Checklist) => void } =
		$props();

	const states: ChecklistItemState[] = ['todo', 'doing', 'done', 'na'];

	async function cycleState(itemId: string, current: ChecklistItemState) {
		const next = states[(states.indexOf(current) + 1) % states.length];
		const updated = await updateChecklistItemState(itemId, next);
		const newChecklist: Checklist = {
			...checklist,
			items: checklist.items.map((i) => (i.id === itemId ? updated : i))
		};
		onchange?.(newChecklist);
	}
</script>

<div class="checklist">
	<h3>{checklist.name}</h3>
	<ul>
		{#each checklist.items as item (item.id)}
			<li>
				<button class="state state-{item.state}" onclick={() => cycleState(item.id, item.state)}>
					{item.state}
				</button>
				<span class:done={item.state === 'done'} class:na={item.state === 'na'}>{item.text}</span>
			</li>
		{/each}
	</ul>
</div>

<style>
	.checklist {
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.75rem;
		margin-bottom: 1rem;
	}
	.checklist h3 {
		margin-top: 0;
	}
	ul {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	li {
		display: flex;
		align-items: center;
		gap: 0.6rem;
	}
	.state {
		font-size: 0.7rem;
		text-transform: uppercase;
		border: 1px solid var(--border-strong);
		border-radius: 999px;
		padding: 0.15rem 0.5rem;
		cursor: pointer;
		width: 4.5rem;
		text-align: center;
	}
	.state-todo {
		background: var(--surface-2);
	}
	.state-doing {
		background: var(--warning-bg);
		border-color: var(--warning);
	}
	.state-done {
		background: var(--success-bg);
		border-color: var(--success);
	}
	.state-na {
		background: var(--surface-2);
		color: var(--text-muted);
	}
	.done {
		text-decoration: line-through;
		color: var(--text-muted);
	}
	.na {
		color: var(--text-faint);
	}
</style>
