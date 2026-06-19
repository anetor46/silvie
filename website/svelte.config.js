import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const config = {
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter({
			// Served by Cloudflare Pages for any 404 response.
			fallback: '404.html',
		}),
		files: {
			// Share static assets (favicon, logo) with the main app — single source of truth.
			assets: '../static',
		},
	},
};

export default config;
