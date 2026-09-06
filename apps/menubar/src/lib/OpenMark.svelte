<!-- the row leaves the app rather than pushing a view, so it is not a chevron -->
<svg
  class="open"
  viewBox="0 0 12 12"
  width="12"
  height="12"
  aria-hidden="true"
  fill="none"
  stroke="currentColor"
  stroke-width="1.2"
>
  <path d="M9.5 6.9v2.6H2.5V2.5h2.6" />
  <path d="M7.2 2.5h2.3v2.3M9.5 2.5 5.7 6.3" stroke-linecap="square" />
</svg>

<style>
  .open {
    flex: none;
    color: var(--row-mark, var(--slate));
    transition: color var(--dur-state) ease-out;
  }
</style>
