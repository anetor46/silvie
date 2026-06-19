<script lang="ts">
	import { page } from '$app/state';
	import SiteLogo from './SiteLogo.svelte';

	function handleCtaClick(e: MouseEvent) {
		// On the home page, smooth-scroll to the contact section.
		// On other pages, let the browser navigate via the href.
		if (page.url.pathname === '/') {
			e.preventDefault();
			document.getElementById('contact')?.scrollIntoView({ behavior: 'smooth' });
		}
	}
</script>

<nav class="nav">
	<a href="/" class="logo-link" aria-label="Silvie home">
		<SiteLogo size={30} />
	</a>
	<a href="/#contact" class="cta" onclick={handleCtaClick}>Get notified</a>
</nav>

<style>
	.nav {
		position: sticky;
		top: 0;
		z-index: 100;
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 14px 6%;
		background: rgba(255, 255, 255, 0.92);
		backdrop-filter: blur(12px);
		-webkit-backdrop-filter: blur(12px);
		border-bottom: 0.5px solid var(--border-light);
	}

	.logo-link {
		text-decoration: none;
		color: inherit;
		display: inline-flex;
	}

	.cta {
		font-family: inherit;
		font-size: 13px;
		font-weight: 500;
		padding: 7px 18px;
		border: 1px solid var(--purple-400);
		border-radius: 20px;
		color: var(--purple-600);
		background: transparent;
		cursor: pointer;
		text-decoration: none;
		transition: background 0.15s;
	}

	.cta:hover {
		background: var(--purple-50);
	}
</style>
