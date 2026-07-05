/**
 * Positions a fixed-position popup/menu at `pos`, nudged back onto-screen if
 * it would otherwise overflow the right or bottom edge of the viewport.
 * Re-clamps on every resize of the element itself (a ResizeObserver, rather
 * than a caller-supplied dependency list, so it stays correct automatically
 * when a menu grows -- e.g. switching from a short list to a bigger form --
 * without every caller needing to remember to list that as a dependency).
 */
export function clampToViewport(node: HTMLElement, pos: { x: number; y: number }) {
	function reposition() {
		const rect = node.getBoundingClientRect();
		const overflowX = rect.right - window.innerWidth;
		const overflowY = rect.bottom - window.innerHeight;
		node.style.left = `${overflowX > 0 ? Math.max(0, pos.x - overflowX) : pos.x}px`;
		node.style.top = `${overflowY > 0 ? Math.max(0, pos.y - overflowY) : pos.y}px`;
	}

	reposition();
	const observer = new ResizeObserver(reposition);
	observer.observe(node);

	return {
		update(newPos: { x: number; y: number }) {
			pos = newPos;
			reposition();
		},
		destroy() {
			observer.disconnect();
		}
	};
}
