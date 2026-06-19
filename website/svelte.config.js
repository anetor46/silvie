import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const config = {
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter({
			fallback: 'index.html',
		}),
		files: {
			// Share static assets (favicon, logo) with the main app — single source of truth.
			assets: '../static',
		},
	},
};

export default config;
