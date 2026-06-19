<script lang="ts">
	import SiteLogo from './SiteLogo.svelte';

	const year = new Date().getFullYear();
</script>

<footer class="footer">
	<div class="inner">
		<a href="/" class="logo-link" aria-label="Silvie home">
			<SiteLogo size={24} />
		</a>
		<nav class="links" aria-label="Footer">
			<a href="/terms" class="link">Terms</a>
		</nav>
		<p class="copy">© {year} Silvie. All rights reserved.</p>
	</div>
</footer>

<style>
	.footer {
		background: var(--purple-50);
		border-top: 0.5px solid var(--border-light);
		padding: 24px 6%;
	}

	.inner {
		max-width: var(--section-max);
		margin: 0 auto;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
	}

	.logo-link {
		text-decoration: none;
		color: inherit;
		display: inline-flex;
	}

	.links {
		display: flex;
		gap: 18px;
	}

	.link {
		font-size: 12px;
		color: var(--text-secondary);
		text-decoration: none;
		transition: color 0.15s;
	}

	.link:hover {
		color: var(--purple-600);
	}

	.copy {
		font-size: 12px;
		color: var(--text-muted);
	}
</style>
