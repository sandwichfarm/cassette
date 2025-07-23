<script>
  import StudioConsole from './lib/components/StudioConsole.svelte';
</script>

<main class="studio">
  <StudioConsole />
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background: #1a1a1a;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    overflow-x: hidden;
  }
  
  .studio {
    min-height: 100vh;
    min-height: 100dvh; /* Dynamic viewport height for better mobile support */
    width: 100vw;
    display: flex;
    align-items: stretch;
    justify-content: center;
    padding: clamp(8px, 2vw, 20px);
    background: 
      radial-gradient(ellipse at top, #2a2a2a 0%, #1a1a1a 50%),
      #1a1a1a;
    box-sizing: border-box;
  }
</style>