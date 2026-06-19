<script lang="ts">
	const badges = [
		{ label: 'App Store', sub: 'Download on the', icon: 'apple' },
		{ label: 'Google Play', sub: 'Get it on', icon: 'play' },
		{ label: 'macOS', sub: 'Available on', icon: 'mac' },
		{ label: 'Windows', sub: 'Available on', icon: 'windows' },
	] as const;
</script>

<section class="hero">
	<div class="inner">
		<span class="badge">
			<span class="dot"></span>
			In development — coming soon
		</span>

		<h1>The <em>personal assistant</em><br />your team never had.</h1>

		<p class="tagline">
			Silvie gives every salesperson, consultant and traveling employee their own AI assistant —
			one that quietly handles trips, meetings, bookings, receipts and follow-ups, so they
			can focus on the work that matters.
		</p>

		<div class="stores">
			{#each badges as { label, sub, icon }}
				<div class="store-badge" title="Coming soon">
					<span class="store-icon" aria-hidden="true">
						{#if icon === 'apple' || icon === 'mac'}
							<svg width="16" height="16" viewBox="0 0 24 24" fill="var(--purple-600)" xmlns="http://www.w3.org/2000/svg">
								<path d="M18.71 19.5c-.83 1.24-1.71 2.45-3.05 2.47-1.34.03-1.77-.79-3.29-.79-1.53 0-2 .77-3.27.82-1.31.05-2.3-1.32-3.14-2.53C4.25 17 2.94 12.45 4.7 9.39c.87-1.52 2.43-2.48 4.12-2.51 1.28-.02 2.5.87 3.29.87.78 0 2.26-1.07 3.8-.91.65.03 2.47.26 3.64 1.98-.09.06-2.17 1.28-2.15 3.81.03 3.02 2.65 4.03 2.68 4.04-.03.07-.42 1.44-1.38 2.83M13 3.5c.73-.83 1.94-1.46 2.94-1.5.13 1.17-.34 2.35-1.04 3.19-.69.85-1.83 1.51-2.95 1.42-.15-1.15.41-2.35 1.05-3.11z"/>
							</svg>
						{:else if icon === 'play'}
							<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--purple-600)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">
								<path d="M3 3l7.07 16.97 2.51-7.39 7.39-2.51L3 3z"/>
								<path d="M13 13l6 6"/>
							</svg>
						{:else if icon === 'windows'}
							<svg width="16" height="16" viewBox="0 0 24 24" fill="var(--purple-600)" xmlns="http://www.w3.org/2000/svg">
								<path d="M3 5.557L10.003 4.5l.002 6.88-7-.042L3 5.557zm7 6.75l.003 6.888L3 18.148v-5.83l7-.011zm.84-8.344L20.999 2.5V11h-10.16V3.963zm10.16 7.848V20.5l-10.16-1.553V11.81h10.16z"/>
							</svg>
						{/if}
					</span>
					<div class="store-text">
						<span class="store-sub">{sub}</span>
						<span class="store-label">{label}</span>
					</div>
					<span class="soon-pill">Soon</span>
				</div>
			{/each}
		</div>
	</div>
</section>

<style>
	.hero {
		background: #ffffff;
		padding: 80px 6% 88px;
		text-align: center;
	}

	.inner {
		max-width: var(--section-max);
		margin: 0 auto;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 24px;
	}

	.badge {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		background: var(--purple-50);
		color: var(--purple-600);
		font-size: 12px;
		font-weight: 500;
		padding: 5px 14px;
		border-radius: 20px;
		border: 1px solid var(--border);
	}

	.dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--purple-400);
		animation: pulse 2s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.4; }
	}

	h1 {
		font-size: clamp(28px, 4.5vw, 48px);
		font-weight: 600;
		line-height: 1.15;
		color: var(--text-primary);
		letter-spacing: -0.02em;
	}

	h1 em {
		font-style: normal;
		color: var(--purple-600);
	}

	.tagline {
		font-size: clamp(15px, 2vw, 18px);
		color: var(--text-secondary);
		max-width: 520px;
		line-height: 1.7;
	}

	.stores {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: 12px;
		margin-top: 8px;
	}

	.store-badge {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 16px;
		border: 1px solid var(--border);
		border-radius: 12px;
		background: #ffffff;
		color: var(--text-primary);
		transition: border-color 0.15s, background 0.15s;
		cursor: default;
	}

	.store-badge:hover {
		border-color: var(--purple-400);
		background: var(--purple-50);
	}

	.store-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.store-text {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 1px;
	}

	.store-sub {
		font-size: 10px;
		color: var(--text-muted);
		line-height: 1;
	}

	.store-label {
		font-size: 14px;
		font-weight: 500;
		color: var(--text-primary);
		line-height: 1.2;
	}

	.soon-pill {
		font-size: 10px;
		font-weight: 500;
		padding: 2px 7px;
		border-radius: 20px;
		background: var(--purple-100);
		color: var(--purple-800);
		margin-left: 4px;
	}
</style>
