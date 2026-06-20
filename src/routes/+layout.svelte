<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import Header from '$lib/components/Header.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import UserPanel from '$lib/components/UserPanel.svelte';
  import Onboarding from '$lib/components/Onboarding.svelte';
  import { profile } from '$lib/stores/profile.svelte';
  import { payment } from '$lib/stores/payment.svelte';

  let { children } = $props();

  let sidebarOpen = $state(false);
  let userPanelOpen = $state(false);
  let showOnboarding = $state(false);

  onMount(async () => {
    await Promise.all([profile.load(), payment.load()]);
    if (!profile.data) showOnboarding = true;
  });
</script>

{#if showOnboarding}
  <Onboarding ondone={() => (showOnboarding = false)} />
{/if}

<Sidebar bind:open={sidebarOpen} />
<UserPanel bind:open={userPanelOpen} />

<div class="frame">
  <Header
    onMenuClick={() => (sidebarOpen = !sidebarOpen)}
    onUserClick={() => (userPanelOpen = !userPanelOpen)}
  />
  <div class="content">
    {@render children()}
  </div>
</div>

<style>
  .frame {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
</style>
