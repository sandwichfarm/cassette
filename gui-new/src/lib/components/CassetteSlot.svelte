<script>
  import { cassetteState, loadCassetteFile } from '../stores/cassette.js';
  
  let isDragging = false;
  let fileInput;
  
  async function handleFile(file) {
    if (file && file.name.endsWith('.wasm')) {
      await loadCassetteFile(file);
    }
  }
  
  function handleDrop(e) {
    e.preventDefault();
    isDragging = false;
    
    const file = e.dataTransfer.files[0];
    handleFile(file);
  }
  
  function handleDragOver(e) {
    e.preventDefault();
    isDragging = true;
  }
  
  function handleDragLeave(e) {
    e.preventDefault();
    isDragging = false;
  }
  
  function handleFileSelect(e) {
    const file = e.target.files[0];
    handleFile(file);
  }
  
  function handleClick() {
    fileInput.click();
  }
</script>

<div 
  class="cassette-slot {isDragging ? 'dragging' : ''} {$cassetteState.loaded ? 'loaded' : ''}"
  on:drop={handleDrop}
  on:dragover={handleDragOver}
  on:dragleave={handleDragLeave}
  on:click={handleClick}
  role="button"
  tabindex="0"
  on:keydown={(e) => e.key === 'Enter' && handleClick()}
>
  {#if $cassetteState.loaded}
    <div class="cassette-label">
      <div class="label-header">NOSTR CASSETTE</div>
      <div class="label-title">{$cassetteState.name || 'UNTITLED'}</div>
      <div class="label-info">
        <span>{$cassetteState.eventCount || 0} EVENTS</span>
        <span>•</span>
        <span>v{$cassetteState.version || '1.0'}</span>
      </div>
      <div class="label-description">{$cassetteState.description || 'No description'}</div>
    </div>
  {:else}
    <div class="empty-slot">
      <div class="slot-icon">📼</div>
      <div class="slot-text">
        {isDragging ? 'DROP CASSETTE' : 'CLICK OR DROP .wasm FILE'}
      </div>
    </div>
  {/if}
</div>

<input 
  type="file" 
  accept=".wasm"
  bind:this={fileInput}
  on:change={handleFileSelect}
  class="hidden"
/>

<style>
  .cassette-slot {
    width: 100%;
    height: 80px;
    background: #333;
    border: 2px solid #555;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.2s ease;
    overflow: hidden;
    position: relative;
  }

  .cassette-slot:hover {
    border-color: #777;
    background: #3a3a3a;
  }

  .cassette-slot.dragging {
    border-color: #4a9eff;
    background: #3a4a5a;
    box-shadow: 0 0 10px rgba(74, 158, 255, 0.3);
  }

  .cassette-slot.loaded {
    background: linear-gradient(135deg, #d3d3d3 0%, #e8e8e8 100%);
    border-color: #666;
  }

  .empty-slot {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: #666;
  }

  .slot-icon {
    font-size: 24px;
    margin-bottom: 4px;
  }

  .slot-text {
    font-size: 12px;
    text-align: center;
  }

  .cassette-label {
    padding: 8px 12px;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    background: linear-gradient(135deg, #f0f0f0 0%, #ffffff 100%);
    color: #333;
    font-family: monospace;
  }

  .label-header {
    font-size: 10px;
    font-weight: bold;
    color: #666;
  }

  .label-title {
    font-size: 14px;
    font-weight: bold;
    text-transform: uppercase;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .label-info {
    font-size: 10px;
    color: #666;
    display: flex;
    gap: 8px;
  }

  .label-description {
    font-size: 10px;
    color: #888;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .hidden {
    display: none;
  }
</style>