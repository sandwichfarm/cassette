<script>
  import { cassettes, selectedCassettes, currentCassette, ejectCassette, loadCassetteFile } from '../stores/cassette.js';
  
  let fileInput;
  
  function handleFileSelect(e) {
    const files = Array.from(e.target.files);
    files.forEach(file => {
      if (file.name.endsWith('.wasm')) {
        loadCassetteFile(file);
      }
    });
    fileInput.value = '';
  }
  
  function toggleSelection(cassetteId) {
    selectedCassettes.update(set => {
      const newSet = new Set(set);
      if (newSet.has(cassetteId)) {
        newSet.delete(cassetteId);
      } else {
        newSet.add(cassetteId);
      }
      return newSet;
    });
  }
  
  function selectAll() {
    selectedCassettes.update(() => new Set(Array.from($cassettes.keys())));
  }
  
  function deselectAll() {
    selectedCassettes.update(() => new Set());
  }
  
  function setCurrent(cassetteData) {
    currentCassette.set(cassetteData);
  }
</script>

<div class="cassette-rack">
  <h3 class="rack-title">CASSETTE LIBRARY</h3>
  
  <div class="controls">
    <button class="control-btn" on:click={() => fileInput.click()}>
      LOAD CASSETTES
    </button>
    <input 
      type="file" 
      accept=".wasm"
      multiple
      bind:this={fileInput}
      on:change={handleFileSelect}
      class="hidden"
    />
  </div>
  
  {#if $cassettes.size > 0}
    <div class="selection-controls">
      <button class="mini-btn" on:click={selectAll}>ALL</button>
      <button class="mini-btn" on:click={deselectAll}>NONE</button>
    </div>
  {/if}
  
  <div class="cassette-list">
    {#if $cassettes.size === 0}
      <div class="empty-state">
        NO CASSETTES LOADED
      </div>
    {:else}
      {#each [...$cassettes.values()] as cassette (cassette.id)}
        <div 
          class="cassette-item {$currentCassette?.id === cassette.id ? 'current' : ''} {$selectedCassettes.has(cassette.id) ? 'selected' : ''}"
        >
          <button 
            class="cassette-main"
            on:click={() => setCurrent(cassette)}
          >
            <div class="cassette-label">
              <div class="name">{cassette.name}</div>
              <div class="info">v{cassette.version} • {cassette.eventCount} events</div>
            </div>
          </button>
          
          <div class="cassette-actions">
            <button 
              class="action-btn"
              on:click={() => toggleSelection(cassette.id)}
              title="Toggle selection"
            >
              {$selectedCassettes.has(cassette.id) ? '■' : '□'}
            </button>
            <button 
              class="action-btn eject"
              on:click={() => ejectCassette(cassette.id)}
              title="Eject"
            >
              ⏏
            </button>
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .cassette-rack {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  
  .rack-title {
    font-family: 'Orbitron', monospace;
    font-size: 18px;
    color: #0f0;
    margin: 0 0 20px 0;
    text-align: center;
    letter-spacing: 0.1em;
  }
  
  .controls {
    margin-bottom: 10px;
  }
  
  .control-btn {
    width: 100%;
    background: transparent;
    border: 2px solid #0f0;
    color: #0f0;
    padding: 10px;
    font-family: 'Share Tech Mono', monospace;
    font-size: 14px;
    cursor: pointer;
    transition: all 0.2s;
    letter-spacing: 0.1em;
  }
  
  .control-btn:hover {
    background: #0f0;
    color: #000;
    box-shadow: 0 0 10px #0f0;
  }
  
  .selection-controls {
    display: flex;
    gap: 10px;
    margin-bottom: 10px;
  }
  
  .mini-btn {
    flex: 1;
    background: transparent;
    border: 1px solid #0a0;
    color: #0a0;
    padding: 5px;
    font-family: 'Share Tech Mono', monospace;
    font-size: 12px;
    cursor: pointer;
    transition: all 0.2s;
  }
  
  .mini-btn:hover {
    background: #0a0;
    color: #000;
  }
  
  .cassette-list {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }
  
  .empty-state {
    text-align: center;
    padding: 40px 20px;
    color: #0a0;
    opacity: 0.5;
    font-size: 14px;
  }
  
  .cassette-item {
    display: flex;
    margin-bottom: 10px;
    border: 1px solid #0a0;
    background: rgba(0, 255, 0, 0.05);
    transition: all 0.2s;
  }
  
  .cassette-item.current {
    border-color: #0f0;
    background: rgba(0, 255, 0, 0.1);
    box-shadow: 0 0 10px rgba(0, 255, 0, 0.3);
  }
  
  .cassette-item.selected {
    background: rgba(0, 255, 0, 0.15);
  }
  
  .cassette-main {
    flex: 1;
    background: transparent;
    border: none;
    color: #0f0;
    padding: 10px;
    text-align: left;
    cursor: pointer;
    font-family: 'Share Tech Mono', monospace;
  }
  
  .cassette-label .name {
    font-size: 14px;
    font-weight: bold;
    margin-bottom: 4px;
  }
  
  .cassette-label .info {
    font-size: 11px;
    color: #0a0;
    opacity: 0.8;
  }
  
  .cassette-actions {
    display: flex;
    align-items: center;
  }
  
  .action-btn {
    background: transparent;
    border: none;
    color: #0f0;
    width: 30px;
    height: 30px;
    cursor: pointer;
    font-size: 16px;
    transition: all 0.2s;
  }
  
  .action-btn:hover {
    color: #0f0;
    text-shadow: 0 0 5px #0f0;
  }
  
  .action-btn.eject:hover {
    color: #f00;
    text-shadow: 0 0 5px #f00;
  }
  
  .hidden {
    display: none;
  }
  
  /* Custom scrollbar */
  .cassette-list::-webkit-scrollbar {
    width: 8px;
  }
  
  .cassette-list::-webkit-scrollbar-track {
    background: rgba(0, 255, 0, 0.1);
  }
  
  .cassette-list::-webkit-scrollbar-thumb {
    background: #0a0;
    border-radius: 4px;
  }
  
  .cassette-list::-webkit-scrollbar-thumb:hover {
    background: #0f0;
  }
</style>