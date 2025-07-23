<script>
  import { createEventDispatcher } from 'svelte';
  
  export let num;
  export let cassette = null;
  export let active = false;
  export let selected = false;
  
  const dispatch = createEventDispatcher();
  
  function handleClick() {
    if (cassette) {
      dispatch('select');
    }
  }
  
  function handleEject(e) {
    e.stopPropagation();
    if (cassette) {
      dispatch('eject');
    }
  }
</script>

<button 
  class="bank-button"
  class:loaded={cassette}
  class:active={active}
  class:selected={selected}
  on:click={handleClick}
>
  <div class="button-number">{num}</div>
  
  {#if cassette}
    <div class="cassette-indicator">
      <div class="indicator-led"></div>
      <div class="cassette-label">{cassette.name.substring(0, 8)}</div>
    </div>
    
    <button 
      class="eject-button"
      on:click={handleEject}
      title="Eject cassette"
    >
      ⏏
    </button>
  {:else}
    <div class="empty-slot">EMPTY</div>
  {/if}
</button>

<style>
  .bank-button {
    position: relative;
    width: 100%;
    aspect-ratio: 1;
    background: linear-gradient(135deg, #3a3a3a 0%, #2a2a2a 100%);
    border: 2px solid #444;
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s;
    padding: 8px;
    overflow: hidden;
    box-shadow: 
      0 2px 4px rgba(0, 0, 0, 0.3),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
  }
  
  /* Pressed effect */
  .bank-button:active {
    transform: scale(0.95);
    box-shadow: 
      0 1px 2px rgba(0, 0, 0, 0.3),
      inset 0 2px 4px rgba(0, 0, 0, 0.5);
  }
  
  /* States */
  .bank-button.loaded {
    border-color: #666;
    background: linear-gradient(135deg, #4a4a4a 0%, #3a3a3a 100%);
  }
  
  .bank-button.active {
    border-color: #0f0;
    box-shadow: 
      0 0 10px rgba(0, 255, 0, 0.3),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
  }
  
  .bank-button.selected {
    background: linear-gradient(135deg, #1a3a1a 0%, #0a2a0a 100%);
  }
  
  /* Button number */
  .button-number {
    position: absolute;
    top: 4px;
    left: 4px;
    font-size: 16px;
    font-weight: bold;
    color: #888;
    text-shadow: 1px 1px 0 #000;
  }
  
  /* Empty slot */
  .empty-slot {
    font-size: 10px;
    color: #555;
    letter-spacing: 0.1em;
  }
  
  /* Cassette indicator */
  .cassette-indicator {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }
  
  .indicator-led {
    width: 8px;
    height: 8px;
    background: #080;
    border-radius: 50%;
    box-shadow: inset 0 0 2px rgba(0, 0, 0, 0.5);
  }
  
  .bank-button.active .indicator-led {
    background: #0f0;
    box-shadow: 
      0 0 8px #0f0,
      inset 0 0 2px rgba(0, 0, 0, 0.5);
  }
  
  .cassette-label {
    font-size: 9px;
    color: #ccc;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    text-align: center;
    width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  
  /* Eject button */
  .eject-button {
    position: absolute;
    bottom: 4px;
    right: 4px;
    width: 20px;
    height: 20px;
    background: #222;
    border: 1px solid #444;
    border-radius: 2px;
    color: #888;
    font-size: 10px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 0.2s;
  }
  
  .bank-button:hover .eject-button {
    opacity: 1;
  }
  
  .eject-button:hover {
    background: #333;
    color: #aaa;
  }
  
  .eject-button:active {
    transform: scale(0.9);
  }
</style>