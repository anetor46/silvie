<script lang="ts">
  let { size = 28, radius = 7 }: { size?: number; radius?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 64 64"
  fill="none"
  xmlns="http://www.w3.org/2000/svg"
  aria-label="Silvie"
>
  <rect width="64" height="64" rx={radius * (64 / size)} fill="var(--purple-600)" />
  <path
    d="M44 22.5 C44 18 40 16 32 16 C24 16 20 19 20 24 C20 33 44 30 44 40 C44 46 39 49 32 49 C25 49 20 46 20 42"
    stroke="#ffffff"
    stroke-width="3"
    stroke-linecap="round"
    fill="none"
  />
</svg>
