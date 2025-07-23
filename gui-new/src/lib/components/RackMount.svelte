<script>
  import PresetPanel from './PresetPanel.svelte';
  import DataDisplay from './DataDisplay.svelte';
  import DetailScreen from './DetailScreen.svelte';
  import { cassettes, selectedCassettes, sendRequest } from '../stores/cassette.js';
  import { soundEffects } from '../stores/sound.js';
  
  let selectedPreset = null;
  let selectedItem = null;
  let currentPage = 0;
  let itemsPerPage = 9;
  
  // Preset configurations
  const presets = [
    { id: 'profiles', label: 'PROFILES', kind: 0 },
    { id: 'relay_lists', label: 'RELAY LISTS', kind: 10002 },
    { id: 'notes', label: 'NOTES', kind: 1 },
    { id: 'replies', label: 'REPLIES', kind: 1, tags: [['e']] },
    { id: 'reactions', label: 'REACTIONS', kind: 7 },
  ];
  
  function selectPreset(preset) {
    selectedPreset = preset;
    currentPage = 0;
    selectedItem = null;
    soundEffects.playButton();
  }
  
  function selectItem(item) {
    selectedItem = item;
    soundEffects.playClick();
  }
  
  function nextPage() {
    currentPage++;
    soundEffects.playClick();
  }
  
  function prevPage() {
    if (currentPage > 0) {
      currentPage--;
      soundEffects.playClick();
    }
  }
</script>

<div class="rack-mount">
  <!-- Rack mount frame -->
  <div class="rack-frame">
    <div class="mounting-holes top">
      {#each Array(8) as _, i}
        <div class="mounting-hole"></div>
      {/each}
    </div>
    
    <div class="rack-content">
      <!-- Preset panel -->
      <PresetPanel 
        {presets} 
        {selectedPreset}
        on:select={(e) => selectPreset(e.detail)}
      />
      
      <!-- Data display panel -->
      <DataDisplay 
        preset={selectedPreset}
        {currentPage}
        {itemsPerPage}
        on:select={(e) => selectItem(e.detail)}
        on:nextPage={nextPage}
        on:prevPage={prevPage}
      />
      
      <!-- Detail screen -->
      <DetailScreen item={selectedItem} />
    </div>
    
    <div class="mounting-holes bottom">
      {#each Array(8) as _, i}
        <div class="mounting-hole"></div>
      {/each}
    </div>
  </div>
</div>

<style>
  .rack-mount {
    width: 100%;
    max-width: 1400px;
    margin: 0 auto;
  }
  
  .rack-frame {
    background: #1a1a1a;
    border: 2px solid #444;
    border-radius: 4px;
    box-shadow: 
      0 10px 40px rgba(0, 0, 0, 0.5),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
  }
  
  .mounting-holes {
    display: flex;
    justify-content: space-between;
    padding: 10px 20px;
    background: linear-gradient(
      to bottom,
      #3a3a3a 0%,
      #2a2a2a 100%
    );
  }
  
  .mounting-holes.top {
    border-bottom: 1px solid #222;
  }
  
  .mounting-holes.bottom {
    border-top: 1px solid #555;
  }
  
  .mounting-hole {
    width: 10px;
    height: 10px;
    background: #111;
    border-radius: 50%;
    box-shadow: 
      inset 0 1px 2px rgba(0, 0, 0, 0.8),
      0 1px 0 rgba(255, 255, 255, 0.1);
  }
  
  .rack-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
    background: #0a0a0a;
    padding: 2px;
  }
</style>