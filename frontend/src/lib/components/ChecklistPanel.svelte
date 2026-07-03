<script lang="ts">
	import type { Checklist } from '$lib/api/checklists';
	import { updateChecklistItem } from '$lib/api/checklists';

	type ChecklistItemState = 'todo' | 'doing' | 'done' | 'na';
	type ChecklistItemAssessment = 'safe' | 'undecided' | 'exploit';

	let { checklist, onchange }: { checklist: Checklist; onchange?: (checklist: Checklist) => void } =
		$props();

	const states: ChecklistItemState[] = ['todo', 'doing', 'done', 'na'];

	async function apply(itemId: string, state: string, assessment: string) {
		const updated = await updateChecklistItem(itemId, state, assessment);
		const newChecklist: Checklist = {
			...checklist,
			items: checklist.items.map((i) => (i.id === itemId ? updated : i))
		};
		onchange?.(newChecklist);
	}

	function cycleState(itemId: string, current: ChecklistItemState, assessment: string) {
		const next = states[(states.indexOf(current) + 1) % states.length];
		apply(itemId, next, assessment);
	}

	// Safe/EXPLOIT both auto-complete the item; Undecided leaves the current state alone.
	function changeAssessment(
		itemId: string,
		currentState: ChecklistItemState,
		assessment: ChecklistItemAssessment
	) {
		const state = assessment === 'safe' || assessment === 'exploit' ? 'done' : currentState;
		apply(itemId, state, assessment);
	}
</script>

<div class="checklist">
	<h3>{checklist.name}</h3>
	<ul>
		{#each checklist.items as item (item.id)}
			<li>
				<button
					class="state state-{item.state}"
					class:assessment-exploit={item.assessment === 'exploit'}
					onclick={() => cycleState(item.id, item.state, item.assessment)}
				>
					{item.state}
				</button>
				<select
					class="assessment"
					value={item.assessment}
					onchange={(e) =>
						changeAssessment(
							item.id,
							item.state,
							(e.target as HTMLSelectElement).value as ChecklistItemAssessment
						)}
				>
					<option value="undecided">Undecided</option>
					<option value="safe">Safe</option>
					<option value="exploit">EXPLOIT</option>
				</select>
				<span
					class:done={item.state === 'done'}
					class:na={item.state === 'na'}
					class:exploit={item.assessment === 'exploit'}>{item.text}</span
				>
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
	.state.assessment-exploit {
		background: var(--error-bg);
		border-color: var(--error);
		color: var(--error);
	}
	.assessment {
		font-size: 0.75rem;
		width: 7rem;
	}
	.done {
		text-decoration: line-through;
		color: var(--text-muted);
	}
	.na {
		color: var(--text-faint);
	}
	.exploit {
		color: var(--error);
		font-weight: 700;
	}
</style>
