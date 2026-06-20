<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import Header from '$lib/components/Header.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import UserPanel from '$lib/components/UserPanel.svelte';
  import Login from '$lib/components/Login.svelte';
  import { auth } from '$lib/stores/auth.svelte';
  import { user } from '$lib/stores/user.svelte';
  import { profile } from '$lib/stores/profile.svelte';
  import { payment } from '$lib/stores/payment.svelte';

  let { children } = $props();

  let sidebarOpen = $state(false);
  let userPanelOpen = $state(false);

  onMount(async () => {
    await auth.load();
    if (auth.user) {
      try {
        await user.loadFromBackend();
        if (!user.record) {
          throw new Error('No backend record found. Please sign up.');
        }
      } catch (e) {
        // Couldn't reach backend or no DB row — push the user back to Login
        // with a clear message instead of half-loading the app.
        auth.user = null;
        user.reset();
        auth.error = e instanceof Error ? e.message : String(e);
        return;
      }
      await Promise.all([profile.load(), payment.load()]);
    }
  });

  // Whenever auth.user flips from null → user (post-login), the auth store
  // has already loaded/synced the backend record. Run the remaining loads.
  let lastUserSub = $state<string | null>(null);
  $effect(() => {
    const sub = auth.user?.sub ?? null;
    if (sub && sub !== lastUserSub && user.record) {
      lastUserSub = sub;
      void Promise.all([profile.load(), payment.load()]);
    } else if (!sub) {
      lastUserSub = null;
    }
  });
</script>

{#if !auth.loaded}
  <div class="boot-splash"></div>
{:else if !auth.user || !user.record}
  <Login />
{:else}
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
{/if}

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

  .boot-splash {
    position: fixed;
    inset: 0;
    background: var(--bg);
  }
</style>
