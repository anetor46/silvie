<script lang="ts">
	const lastUpdated = '19 June 2026';
</script>

<svelte:head>
	<title>Terms of Service — Silvie</title>
	<meta
		name="description"
		content="Terms of Service for the Silvie marketing website. Silvie is currently in development."
	/>
</svelte:head>

<article class="page">
	<header class="hero">
		<p class="eyebrow">Legal</p>
		<h1>Terms of Service</h1>
		<p class="updated">Last updated: {lastUpdated}</p>
	</header>

	<section>
		<p class="lead">
			These terms govern your use of the Silvie marketing website at <strong>silvie.uk</strong>.
			Silvie is a software product currently in development; the website exists to share
			information about the product and to let interested users, investors and potential
			collaborators get in touch with us. When the Silvie application becomes available,
			separate terms will apply to its use.
		</p>
	</section>

	<section>
		<h2>1. Who we are</h2>
		<p>
			"Silvie", "we", "us" and "our" refer to the team building the Silvie product. You can
			reach us at <a href="mailto:support@silvie.uk">support@silvie.uk</a>.
		</p>
	</section>

	<section>
		<h2>2. Use of the website</h2>
		<p>
			You may browse the Silvie website for informational and personal purposes. You agree
			not to use the website in any way that is unlawful, that could damage or impair the
			site, or that interferes with anyone else's use of it. You agree not to attempt to
			gain unauthorised access to any part of the website or any related systems.
		</p>
	</section>

	<section>
		<h2>3. Pre-launch product</h2>
		<p>
			Silvie is in active development and is not yet available to download or use.
			Information on this website about features, availability dates, supported platforms
			or capabilities is provided in good faith but is indicative only. We may change,
			delay, or discontinue planned features at any time without notice.
		</p>
	</section>

	<section>
		<h2>4. Contacting us</h2>
		<p>
			The contact form on this site opens your own email client and sends a message to
			<a href="mailto:support@silvie.uk">support@silvie.uk</a>. We do not collect or store
			email addresses entered on the site itself. When you email us, we will use the
			information you send only to respond to your enquiry and to keep you informed about
			Silvie if you have asked us to.
		</p>
	</section>

	<section>
		<h2>5. Intellectual property</h2>
		<p>
			All content on the Silvie website — including text, graphics, the Silvie name and
			logo, page layouts, and source code — is owned by us or our licensors and is
			protected by intellectual-property laws. You may not copy, reproduce, republish or
			redistribute any part of the website without our prior written permission, except for
			personal, non-commercial viewing.
		</p>
	</section>

	<section>
		<h2>6. Third-party links and brands</h2>
		<p>
			The website may reference or link to third-party products and services (for example
			Google Calendar, Outlook, Uber, OpenTable, Concur). These references are descriptive
			only. We are not affiliated with, endorsed by, or sponsored by any of these companies
			unless explicitly stated.
		</p>
	</section>

	<section>
		<h2>7. No warranties</h2>
		<p>
			The website is provided on an "as is" and "as available" basis. To the fullest extent
			permitted by law, we make no warranties or representations of any kind, express or
			implied, regarding the website's accuracy, reliability, completeness or availability.
		</p>
	</section>

	<section>
		<h2>8. Limitation of liability</h2>
		<p>
			To the fullest extent permitted by law, we will not be liable for any indirect,
			incidental, special or consequential damages arising from your use of, or inability
			to use, this website. Nothing in these terms excludes or limits liability that cannot
			lawfully be excluded or limited under English law.
		</p>
	</section>

	<section>
		<h2>9. Changes to these terms</h2>
		<p>
			We may update these terms from time to time. The "last updated" date at the top of
			this page indicates when these terms were last revised. Your continued use of the
			website after any update constitutes acceptance of the new terms.
		</p>
	</section>

	<section>
		<h2>10. Governing law</h2>
		<p>
			These terms are governed by the laws of England and Wales. Any dispute arising from
			or in connection with these terms or your use of the website will be subject to the
			exclusive jurisdiction of the courts of England and Wales.
		</p>
	</section>

	<section>
		<h2>11. Contact</h2>
		<p>
			If you have any questions about these terms, please email
			<a href="mailto:support@silvie.uk">support@silvie.uk</a>.
		</p>
	</section>
</article>

<style>
	.page {
		max-width: 760px;
		margin: 0 auto;
		padding: 56px 6% 80px;
	}

	.hero {
		margin-bottom: 40px;
	}

	.eyebrow {
		font-size: 12px;
		font-weight: 500;
		color: var(--purple-600);
		letter-spacing: 0.08em;
		text-transform: uppercase;
		margin-bottom: 12px;
	}

	h1 {
		font-size: clamp(28px, 4vw, 38px);
		font-weight: 600;
		color: var(--text-primary);
		letter-spacing: -0.02em;
		line-height: 1.15;
		margin-bottom: 8px;
	}

	.updated {
		font-size: 13px;
		color: var(--text-muted);
	}

	section {
		margin-bottom: 28px;
	}

	h2 {
		font-size: 17px;
		font-weight: 600;
		color: var(--text-primary);
		margin-bottom: 10px;
		letter-spacing: -0.01em;
	}

	p {
		font-size: 15px;
		color: var(--text-secondary);
		line-height: 1.75;
	}

	.lead {
		font-size: 16px;
		color: var(--text-secondary);
	}

	a {
		color: var(--purple-600);
		text-decoration: none;
		border-bottom: 1px solid transparent;
		transition: border-color 0.15s;
	}

	a:hover {
		border-bottom-color: var(--purple-400);
	}

	strong {
		color: var(--text-primary);
		font-weight: 500;
	}
</style>
