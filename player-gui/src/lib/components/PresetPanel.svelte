<script>
  import { createEventDispatcher } from 'svelte';
  
  export let presets = [];
  export let selectedPreset = null;
  
  const dispatch = createEventDispatcher();
</script>

<div class="preset-panel">
  <div class="panel-label">PRESETS</div>
  
  <div class="preset-buttons">
    {#each presets as preset}
      <button 
        class="preset-button"
        class:active={selectedPreset?.id === preset.id}
        on:click={() => dispatch('select', preset)}
      >
        <div class="button-led"></div>
        <span class="button-label">{preset.label}</span>
      </button>
    {/each}
    
    <!-- Blank preset buttons for future expansion -->
    {#each Array(3) as _}
      <button class="preset-button blank" disabled>
        <div class="button-led"></div>
        <span class="button-label">EMPTY</span>
      </button>
    {/each}
  </div>
</div>

<style>
  .preset-panel {
    background: #2a2a2a;
    padding: 20px;
    border: 1px solid #333;
  }
  
  .panel-label {
    font-size: 12px;
    color: #888;
    letter-spacing: 0.2em;
    margin-bottom: 15px;
    text-align: center;
  }
  
  .preset-buttons {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 10px;
  }
  
  .preset-button {
    height: 60px;
    background: linear-gradient(to bottom, #3a3a3a, #2a2a2a);
    border: 2px solid #444;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 5px;
    transition: all 0.1s;
    position: relative;
    box-shadow: 
      0 2px 4px rgba(0, 0, 0, 0.3),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
  }
  
  .preset-button:hover:not(:disabled) {
    background: linear-gradient(to bottom, #4a4a4a, #3a3a3a);
    border-color: #555;
  }
  
  .preset-button:active:not(:disabled) {
    transform: translateY(1px);
    box-shadow: 
      0 1px 2px rgba(0, 0, 0, 0.3),
      inset 0 1px 3px rgba(0, 0, 0, 0.5);
  }
  
  .preset-button.active {
    background: linear-gradient(to bottom, #2a2a2a, #1a1a1a);
    border-color: #0f0;
    box-shadow: 
      0 0 10px rgba(0, 255, 0, 0.2),
      inset 0 1px 3px rgba(0, 0, 0, 0.5);
  }
  
  .preset-button.active .button-led {
    background: #0f0;
    box-shadow: 0 0 8px #0f0;
  }
  
  .preset-button.blank {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
  .button-led {
    width: 8px;
    height: 8px;
    background: #333;
    border-radius: 50%;
    transition: all 0.2s;
  }
  
  .button-label {
    font-size: 10px;
    color: #aaa;
    letter-spacing: 0.1em;
    text-align: center;
  }
  
  @media (max-width: 768px) {
    .preset-buttons {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>