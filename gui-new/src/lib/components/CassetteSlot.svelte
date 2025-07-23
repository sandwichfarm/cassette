<script>
  import { loadCassetteFile } from '../utils/cassetteLoader.js';
  import { cassetteStatus, cassetteMetadata, loadError } from '../stores/cassette.js';
  
  let fileInput;
  
  async function handleFileSelect(event) {
    const file = event.target.files[0];
    if (file && file.name.endsWith('.wasm')) {
      try {
        await loadCassetteFile(file);
      } catch (error) {
        console.error('Failed to load cassette:', error);
      }
    }
  }
  
  function handleEject() {
    if (fileInput) {
      fileInput.value = '';
    }
    // Reset will be handled by control buttons
  }
</script>

<div class="cassette-slot bg-black rounded-lg p-6 border-4 border-gray-700 shadow-inner">
  {#if $cassetteStatus === 'empty'}
    <div class="text-center">
      <p class="text-gray-500 mb-4">INSERT CASSETTE</p>
      <label class="cassette-button cursor-pointer">
        LOAD
        <input 
          type="file" 
          accept=".wasm" 
          on:change={handleFileSelect}
          bind:this={fileInput}
          class="hidden"
        />
      </label>
    </div>
  {:else if $cassetteStatus === 'loading'}
    <div class="text-center">
      <div class="animate-pulse">
        <p class="text-yellow-400">LOADING...</p>
        <div class="mt-2 w-full bg-gray-700 rounded-full h-2">
          <div class="bg-yellow-400 h-2 rounded-full animate-pulse" style="width: 45%"></div>
        </div>
      </div>
    </div>
  {:else if $cassetteStatus === 'error'}
    <div class="text-center">
      <p class="text-red-500 mb-2">ERROR</p>
      <p class="text-xs text-gray-500">{$loadError}</p>
      <button class="cassette-button mt-4 bg-red-600" on:click={() => handleEject()}>
        EJECT
      </button>
    </div>
  {:else if $cassetteStatus === 'loaded' && $cassetteMetadata}
    <div class="cassette-label bg-gradient-to-b from-gray-100 to-gray-300 text-black rounded p-4">
      <div class="flex justify-between items-start mb-2">
        <div class="font-mono text-xs">
          <div>CASSETTE TYPE C-90</div>
          <div>DOLBY SYSTEM</div>
        </div>
        <div class="text-xs text-right">
          <div>SIDE A</div>
          <div class="text-red-600">● REC</div>
        </div>
      </div>
      
      <div class="border-t border-b border-gray-600 py-2 my-2">
        <h3 class="font-bold text-sm uppercase truncate">
          {$cassetteMetadata.name || 'UNTITLED'}
        </h3>
        <p class="text-xs truncate">
          {$cassetteMetadata.description || 'No description'}
        </p>
      </div>
      
      <div class="text-xs space-y-1">
        <div class="flex justify-between">
          <span>Events:</span>
          <span class="font-mono">{$cassetteMetadata.event_count || 0}</span>
        </div>
        <div class="flex justify-between">
          <span>Version:</span>
          <span class="font-mono">{$cassetteMetadata.version || '1.0.0'}</span>
        </div>
      </div>
    </div>
  {/if}
</div>