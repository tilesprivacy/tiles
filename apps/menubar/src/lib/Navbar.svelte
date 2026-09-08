<script lang="ts">
  import Chevron from "./Chevron.svelte";

  interface Props {
    title: string;
    onback: () => void;
  }

  let { title, onback }: Props = $props();
</script>

<div class="navbar">
  <button class="navbar__back" aria-label="Back" onclick={onback}>
    <Chevron back />
  </button>
  <span class="navbar__title">{title}</span>
</div>

<style>
  .navbar {
    display: flex;
    align-items: center;
    gap: 8px;
    height: var(--h-nav);
    padding: 0 var(--pad-x);
  }

  .navbar__back {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: none;
    width: 20px;
    height: 20px;
    border: none;
    background: transparent;
  }

  .navbar__back:hover {
    color: var(--signal);
  }

  .navbar__title {
    font-size: var(--fs-title);
    font-weight: 500;
    color: var(--bone);
  }
</style>
