<script lang="ts">
  import BrandMark from './BrandMark.svelte';
</script>

<div class="thinking" role="status" aria-live="polite" aria-label="Assistant is thinking">
  <div class="avatar">
    <BrandMark size={30} radius={7} />
  </div>
  <div class="dots" aria-hidden="true">
    <span class="dot"></span>
    <span class="dot"></span>
    <span class="dot"></span>
  </div>
</div>

<style>
  .thinking {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }

  .avatar {
    width: 30px;
    height: 30px;
    flex-shrink: 0;
    margin-top: 2px;
    line-height: 0;
  }

  .dots {
    display: flex;
    gap: 5px;
    align-items: center;
    padding: 10px 0;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-dim);
    animation: wave 1.3s infinite ease-in-out both;
  }

  .dot:nth-child(1) { animation-delay: 0s; }
  .dot:nth-child(2) { animation-delay: 0.16s; }
  .dot:nth-child(3) { animation-delay: 0.32s; }

  @keyframes wave {
    0%, 70%, 100% {
      transform: translateY(0);
      opacity: 0.35;
    }
    35% {
      transform: translateY(-5px);
      opacity: 1;
    }
  }
</style>
