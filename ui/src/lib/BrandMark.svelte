<script lang="ts">
  // The ClipVault symbol from the master SVGs (specs/07 §1): flat up to 32 px, detailed above.
  import symbol from '../../../assets/brand/logo-symbol.svg?raw';
  import flat from '../../../assets/brand/logo-symbol-flat.svg?raw';

  let { size = 64 }: { size?: number } = $props();
</script>

<span class="mark" style:width="{size}px" style:height="{size}px" aria-hidden="true">
  <!-- eslint-disable-next-line svelte/no-at-html-tags -- static master SVG from the repository -->
  {@html size <= 32 ? flat : symbol}
</span>

<style>
  .mark {
    display: inline-flex;
    flex: none;
  }

  .mark :global(svg) {
    width: 100%;
    height: 100%;
  }
</style>
