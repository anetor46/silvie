<script lang="ts">
	const features = [
		{
			title: 'Every booking, automatically tracked',
			body: 'Flights, hotels, trains, taxis and restaurant reservations — automatically detected from email, organised and enriched with what is still missing. No more spreadsheets, no more forwarded confirmations.',
			icon: 'calendar',
		},
		{
			title: 'Catches what people forget',
			body: 'Spots the hotel missing from a trip, the taxi not booked after landing, the meeting scheduled too close to arrival, and the receipt not yet filed for expenses.',
			icon: 'spark',
		},
		{
			title: 'Built for whole teams',
			body: 'Roll Silvie out to your sales, consulting or field teams. Sits above the tools they already use — Outlook, Google Calendar, Uber, Concur — without replacing them.',
			icon: 'grid',
		},
	] as const;
</script>

<section class="features">
	<div class="inner">
		<p class="eyebrow">What Silvie does</p>
		<h2>One assistant. Every commitment handled.</h2>

		<div class="grid">
			{#each features as { title, body, icon }}
				<div class="card">
					<div class="icon-wrap" aria-hidden="true">
						{#if icon === 'calendar'}
							<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--purple-600)" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
								<rect x="3" y="4" width="18" height="18" rx="2" />
								<path d="M16 2v4M8 2v4M3 10h18" />
							</svg>
						{:else if icon === 'spark'}
							<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--purple-600)" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
								<path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
								<circle cx="12" cy="12" r="3" />
							</svg>
						{:else if icon === 'grid'}
							<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--purple-600)" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
								<rect x="3" y="3" width="7" height="7" rx="1" />
								<rect x="14" y="3" width="7" height="7" rx="1" />
								<rect x="3" y="14" width="7" height="7" rx="1" />
								<rect x="14" y="14" width="7" height="7" rx="1" />
							</svg>
						{/if}
					</div>
					<h3>{title}</h3>
					<p>{body}</p>
				</div>
			{/each}
		</div>
	</div>
</section>

<style>
	.features {
		background: var(--gray-50);
		padding: 80px 6%;
		border-top: 0.5px solid var(--border-light);
		border-bottom: 0.5px solid var(--border-light);
	}

	.inner {
		max-width: var(--section-max);
		margin: 0 auto;
	}

	.eyebrow {
		font-size: 12px;
		font-weight: 500;
		color: var(--purple-600);
		letter-spacing: 0.08em;
		text-transform: uppercase;
		text-align: center;
		margin-bottom: 10px;
	}

	h2 {
		font-size: clamp(20px, 3vw, 28px);
		font-weight: 600;
		color: var(--text-primary);
		text-align: center;
		letter-spacing: -0.01em;
		margin-bottom: 40px;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
		gap: 20px;
	}

	.card {
		background: #ffffff;
		border: 0.5px solid var(--border-light);
		border-radius: 16px;
		padding: 28px 24px;
		transition: border-color 0.15s, box-shadow 0.15s;
	}

	.card:hover {
		border-color: var(--border);
		box-shadow: 0 2px 12px rgba(83, 74, 183, 0.08);
	}

	.icon-wrap {
		width: 40px;
		height: 40px;
		border-radius: 10px;
		background: var(--purple-50);
		display: flex;
		align-items: center;
		justify-content: center;
		margin-bottom: 16px;
	}

	h3 {
		font-size: 15px;
		font-weight: 500;
		color: var(--text-primary);
		margin-bottom: 8px;
		line-height: 1.4;
	}

	p {
		font-size: 14px;
		color: var(--text-secondary);
		line-height: 1.7;
	}
</style>
