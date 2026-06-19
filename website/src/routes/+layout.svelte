<script lang="ts">
	import '../app.css';
	import SiteNav from '$lib/components/SiteNav.svelte';
	import FooterSection from '$lib/components/FooterSection.svelte';

	let { children } = $props();
</script>

<SiteNav />
<main>
	{@render children()}
</main>
<FooterSection />
