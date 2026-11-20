<script lang="ts">
  // Placeholder symbol (M0) — final identity in M6 (specs/07 §1).
  let { size = 64 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 1024 1024" aria-hidden="true">
  <rect x="144" y="176" width="736" height="784" rx="160" fill="var(--brand-600)" />
  <rect x="352" y="80" width="320" height="192" rx="80" fill="var(--vault-teal)" />
  <circle cx="512" cy="164" r="34" fill="#14131f" />
  <rect x="496" y="176" width="32" height="56" rx="12" fill="#14131f" />
  <rect x="288" y="448" width="448" height="64" rx="32" fill="#fff" />
  <rect x="288" y="584" width="352" height="64" rx="32" fill="#fff" fill-opacity="0.75" />
  <rect x="288" y="720" width="256" height="64" rx="32" fill="#fff" fill-opacity="0.5" />
</svg>
