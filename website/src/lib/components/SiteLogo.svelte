<script lang="ts">
	interface Props {
		size?: number;
	}
	let { size = 30 }: Props = $props();

	const fontSize = $derived(Math.round(size * 0.53));
	const gap = $derived(Math.round(size * 0.28));
</script>

<div class="logo" style="gap: {gap}px">
	<svg width={size} height={size} viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Silvie mark">
		<rect width="64" height="64" rx="14" fill="var(--purple-600)" />
		<path
			d="M44 22.5 C44 18 40 16 32 16 C24 16 20 19 20 24 C20 33 44 30 44 40 C44 46 39 49 32 49 C25 49 20 46 20 42"
			stroke="#ffffff"
			stroke-width="3"
			stroke-linecap="round"
			fill="none"
		/>
	</svg>
	<span class="wordmark" style="font-size: {fontSize}px">Silvie</span>
</div>

<style>
	.logo {
		display: flex;
		align-items: center;
	}
	.wordmark {
		font-weight: 500;
		letter-spacing: -0.01em;
		color: var(--text-primary);
	}
</style>
