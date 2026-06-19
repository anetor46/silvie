<script lang="ts">
	const CONTACT_EMAIL = 'support@silvie.uk';

	let email = $state('');
	let submitted = $state(false);

	function handleSubmit(e: Event) {
		e.preventDefault();
		const subject = encodeURIComponent('Silvie — I want to stay in the loop');
		const body = encodeURIComponent(
			`Hi,\n\nI'm interested in Silvie and would love to stay updated.\n\n${email ? `You can reach me at: ${email}\n\n` : ''}`,
		);
		window.location.href = `mailto:${CONTACT_EMAIL}?subject=${subject}&body=${body}`;
		submitted = true;
	}
</script>

<section class="email-section" id="contact">
	<div class="inner">
		<h2>Bring Silvie to your team</h2>
		<p class="sub">
			Silvie is in active development. If you run a sales, consulting or field team —<br
				class="br-desktop"
			/> or you're an investor or want to join the team — we would love to hear from you.
		</p>

		{#if !submitted}
			<form class="form" onsubmit={handleSubmit}>
				<input
					type="email"
					bind:value={email}
					placeholder="your@email.com"
					class="input"
					aria-label="Your email address"
				/>
				<button type="submit" class="btn">Get in touch →</button>
			</form>
			<p class="hint">Opens your email client — no forms, no spam.</p>
		{:else}
			<div class="thanks">
				<span class="check" aria-hidden="true">✓</span>
				<p>Your email client should have opened. We look forward to hearing from you.</p>
			</div>
		{/if}
	</div>
</section>

<style>
	.email-section {
		background: #ffffff;
		padding: 88px 6%;
		text-align: center;
	}

	.inner {
		max-width: 560px;
		margin: 0 auto;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 16px;
	}

	h2 {
		font-size: clamp(22px, 3vw, 30px);
		font-weight: 600;
		color: var(--text-primary);
		letter-spacing: -0.01em;
	}

	.sub {
		font-size: 15px;
		color: var(--text-secondary);
		line-height: 1.7;
	}

	.form {
		display: flex;
		gap: 8px;
		width: 100%;
		max-width: 420px;
		margin-top: 8px;
	}

	.input {
		flex: 1;
		font-family: inherit;
		font-size: 14px;
		padding: 10px 14px;
		border: 1px solid var(--border);
		border-radius: 10px;
		color: var(--text-primary);
		background: #ffffff;
		transition: border-color 0.15s;
		min-width: 0;
	}

	.input::placeholder {
		color: var(--text-muted);
	}

	.input:focus {
		outline: none;
		border-color: var(--purple-400);
	}

	.btn {
		font-family: inherit;
		font-size: 14px;
		font-weight: 500;
		padding: 10px 20px;
		background: var(--purple-600);
		color: #ffffff;
		border: none;
		border-radius: 10px;
		cursor: pointer;
		white-space: nowrap;
		transition: background 0.15s;
	}

	.btn:hover {
		background: var(--purple-800);
	}

	.hint {
		font-size: 12px;
		color: var(--text-muted);
	}

	.thanks {
		display: flex;
		align-items: center;
		gap: 10px;
		background: var(--purple-50);
		border: 1px solid var(--border);
		border-radius: 12px;
		padding: 16px 24px;
		margin-top: 8px;
	}

	.check {
		font-size: 18px;
		color: var(--purple-600);
	}

	.thanks p {
		font-size: 14px;
		color: var(--text-secondary);
	}

	@media (max-width: 520px) {
		.form {
			flex-direction: column;
		}
		.br-desktop {
			display: none;
		}
	}
</style>
