<script lang="ts">
  import '../app.css';
  import Header from '$lib/components/Header.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';

  let { children } = $props();

  let sidebarOpen = $state(false);
</script>

<Sidebar bind:open={sidebarOpen} />

<div class="frame">
  <Header onMenuClick={() => (sidebarOpen = !sidebarOpen)} />
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
