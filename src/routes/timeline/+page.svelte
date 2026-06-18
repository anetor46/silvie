<script lang="ts">
  import { getMockEvents } from '$lib/data/events';
  import EventCard from '$lib/components/EventCard.svelte';

  const events = getMockEvents();

  function startOfDay(d: Date): number {
    const s = new Date(d);
    s.setHours(0, 0, 0, 0);
    return s.getTime();
  }

  const today = startOfDay(new Date());

  function dayLabel(dayKey: number): string {
    const diff = Math.round((dayKey - today) / 86400000);
    if (diff === 0) return 'Today';
    if (diff === 1) return 'Tomorrow';
    if (diff === -1) return 'Yesterday';
    const d = new Date(dayKey);
    return d.toLocaleDateString([], { weekday: 'long', day: 'numeric', month: 'long' });
  }

  // Group events by day, then split into past/upcoming with upcoming first.
  const grouped = $derived(() => {
    const byDay = new Map<number, typeof events>();
    for (const e of events) {
      const key = startOfDay(e.start);
      if (!byDay.has(key)) byDay.set(key, []);
      byDay.get(key)!.push(e);
    }
    const dayKeys = [...byDay.keys()].sort((a, b) => a - b);
    const upcoming = dayKeys.filter((k) => k >= today);
    const past = dayKeys.filter((k) => k < today).reverse();

    return {
      upcoming: upcoming.map((k) => ({
        key: k,
        label: dayLabel(k),
        events: byDay.get(k)!.sort((a, b) => a.start.getTime() - b.start.getTime()),
      })),
      past: past.map((k) => ({
        key: k,
        label: dayLabel(k),
        events: byDay.get(k)!.sort((a, b) => a.start.getTime() - b.start.getTime()),
      })),
    };
  });
</script>

<div class="page">
  <div class="page-header">
    <h1 class="title">Timeline</h1>
    <p class="subtitle">What's next, and what just happened — across trips, meetings and bookings.</p>
  </div>

  <section>
    <h2 class="section-label">Upcoming</h2>
    {#if grouped().upcoming.length === 0}
      <p class="empty">Nothing on the horizon.</p>
    {:else}
      {#each grouped().upcoming as group (group.key)}
        <div class="day">
          <div class="day-label">{group.label}</div>
          <div class="events">
            {#each group.events as event (event.id)}
              <EventCard {event} />
            {/each}
          </div>
        </div>
      {/each}
    {/if}
  </section>

  {#if grouped().past.length > 0}
    <section class="past-section">
      <h2 class="section-label">Past</h2>
      {#each grouped().past as group (group.key)}
        <div class="day">
          <div class="day-label">{group.label}</div>
          <div class="events">
            {#each group.events as event (event.id)}
              <EventCard {event} past />
            {/each}
          </div>
        </div>
      {/each}
    </section>
  {/if}
</div>

<style>
  .page {
    flex: 1;
    overflow-y: auto;
    padding: 32px 24px 48px;
  }

  .page::-webkit-scrollbar {
    width: 6px;
  }

  .page::-webkit-scrollbar-thumb {
    background: #2a2a2a;
    border-radius: 3px;
  }

  .page-header {
    margin-bottom: 28px;
    max-width: 640px;
  }

  .title {
    font-size: 24px;
    font-weight: 700;
    color: #ffffff;
    margin-bottom: 6px;
  }

  .subtitle {
    font-size: 14px;
    color: #555;
  }

  section {
    max-width: 640px;
  }

  .past-section {
    margin-top: 36px;
  }

  .section-label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #444;
    margin-bottom: 14px;
  }

  .day {
    margin-bottom: 22px;
  }

  .day-label {
    font-size: 12px;
    font-weight: 600;
    color: #888;
    padding: 0 4px 8px;
  }

  .events {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .empty {
    font-size: 13px;
    color: #444;
    padding: 8px 4px;
  }
</style>
