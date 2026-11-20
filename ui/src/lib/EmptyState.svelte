<script lang="ts">
  // Empty-history illustration (specs/06 §2.4), drawn in the accent color via currentColor.
  import illustration from '../../../assets/brand/empty-state.svg?raw';
</script>

<span class="illustration" aria-hidden="true">
  <!-- eslint-disable-next-line svelte/no-at-html-tags -- static master SVG from the repository -->
  {@html illustration}
</span>

<style>
  .illustration {
    display: inline-flex;
    width: 96px;
    height: 80px;
    color: var(--accent);
  }

  .illustration :global(svg) {
    width: 100%;
    height: 100%;
  }

  @media (forced-colors: active) {
    .illustration {
      color: CanvasText;
    }
  }
</style>
