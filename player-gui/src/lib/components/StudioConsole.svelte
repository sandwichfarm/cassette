<script>
  import { cassettes, currentCassette, selectedCassettes, sendRequest, terminalOutput, loadCassetteFile, ejectCassette, isPlaying } from '../stores/cassette.js';
  import { soundEnabled, soundEffects } from '../stores/sound.js';
  import { onMount, onDestroy } from 'svelte';
  
  let fileInput;
  let currentBank = null;
  let leftMeterHeight = 30;
  let rightMeterHeight = 30;
  let animationFrame;
  
  const presets = [
    { id: 'all', label: 'ALL', all: true },
    { id: 'profiles', label: 'PROFILES', kind: 0 },
    { id: 'relay_lists', label: 'RELAY LISTS', kind: 10002 },
    { id: 'notes', label: 'NOTES', kind: 1 },
    { id: 'replies', label: 'REPLIES', kind: 1, tags: [['e']] },
    { id: 'reactions', label: 'REACTIONS', kind: 7 },
  ];
  
  // Preset system
  let selectedPreset = presets[0]; // Default to "all"
  let selectedItem = null;
  let currentPage = 0;
  let itemsPerPage = 18;
  let innerWidth = 0;
  let innerHeight = 0;

  // Sorting system
  let sortBy = 'created_at'; // 'created_at' or 'kind'
  let sortOrder = 'desc'; // 'asc' or 'desc'
  
  // Calculate items to fill entire viewport
  $: {
    if (innerWidth && innerHeight) {
      // Estimate available space for data cells
      const headerHeight = 120; // rough estimate for controls
      const cellHeight = 28;
      const cellWidth = 90;
      
      const availableHeight = innerHeight - headerHeight;
      const rows = Math.floor(availableHeight / cellHeight);
      const cols = Math.floor(innerWidth / cellWidth);
      
      itemsPerPage = rows * cols;
      if (itemsPerPage < 20) itemsPerPage = 20; // minimum
      if (itemsPerPage > 200) itemsPerPage = 200; // maximum
    }
  }
  let items = [];
  let loading = false;
  
  // Map cassettes to banks 1-9
  $: cassetteArray = Array.from($cassettes.values());
  $: bankCassettes = cassetteArray.slice(0, 9);
  
  // VU meter animation
  $: if ($isPlaying) {
    startMeterAnimation();
  } else {
    stopMeterAnimation();
  }
  
  function startMeterAnimation() {
    if (animationFrame) return;
    animateMeterFrame();
  }
  
  function stopMeterAnimation() {
    if (animationFrame) {
      cancelAnimationFrame(animationFrame);
      animationFrame = null;
    }
    leftMeterHeight = 30;
    rightMeterHeight = 30;
  }
  
  function animateMeterFrame() {
    if (!$isPlaying) return;
    
    const baseLevel = 50;
    const variance = 30;
    
    leftMeterHeight = baseLevel + (Math.random() - 0.5) * variance;
    rightMeterHeight = baseLevel + (Math.random() - 0.5) * variance;
    
    if (Math.random() < 0.1) {
      leftMeterHeight = 70 + Math.random() * 25;
    }
    if (Math.random() < 0.1) {
      rightMeterHeight = 70 + Math.random() * 25;
    }
    
    leftMeterHeight = Math.max(5, Math.min(95, leftMeterHeight));
    rightMeterHeight = Math.max(5, Math.min(95, rightMeterHeight));
    
    animationFrame = requestAnimationFrame(animateMeterFrame);
  }
  
  // File handling
  function handleFileSelect(e) {
    const files = Array.from(e.target.files);
    files.forEach(file => {
      if (file.name.endsWith('.wasm')) {
        loadCassetteFile(file);
        soundEffects.playLoad();
      }
    });
    fileInput.value = '';
  }
  
  // Bank operations
  function selectBank(num) {
    const cassette = bankCassettes[num - 1];
    if (cassette) {
      currentBank = num;
      currentCassette.set(cassette);
      soundEffects.playButton();
      
      selectedCassettes.update(set => {
        const newSet = new Set(set);
        if (newSet.has(cassette.id)) {
          newSet.delete(cassette.id);
        } else {
          newSet.add(cassette.id);
        }
        return newSet;
      });
    }
  }
  
  function ejectBank(num) {
    const cassette = bankCassettes[num - 1];
    if (cassette) {
      ejectCassette(cassette.id);
      soundEffects.playEject();
      if (currentBank === num) {
        currentBank = null;
      }
    }
  }
  
  // Preset operations
  function selectPreset(preset) {
    selectedPreset = preset;
    currentPage = 0;
    selectedItem = null;
    soundEffects.playButton();
    loadPresetData();
  }
  
  async function loadPresetData() {
    if (!selectedPreset || $cassettes.size === 0) return;
    
    loading = true;
    items = [];
    
    const filter = {};
    if (selectedPreset.all) {
      // "all" preset - no filter restriction, just pagination
    } else if (selectedPreset.kind !== undefined) {
      filter.kinds = [selectedPreset.kind];
    }
    if (selectedPreset.tags) {
      filter.tags = selectedPreset.tags;
    }
    
    filter.limit = itemsPerPage;
    filter.offset = currentPage * itemsPerPage;
    
    const request = JSON.stringify(['REQ', `preset-${selectedPreset.id}-${Date.now()}`, filter]);
    
    try {
      const firstCassette = Array.from($cassettes.values())[0];
      if (!firstCassette) return;
      
      const response = firstCassette.cassette.methods.req(request);
      const events = JSON.parse(response);
      
      items = events.slice(0, itemsPerPage).map(event => {
        const item = {
          id: event.id,
          kind: event.kind,
          created_at: event.created_at,
          pubkey: event.pubkey,
          content: event.content,
          tags: event.tags
        };
        
        if (event.kind === 0) {
          try {
            const metadata = JSON.parse(event.content);
            item.name = metadata.name || metadata.display_name || 'Anonymous';
            item.picture = metadata.picture;
            item.about = metadata.about;
          } catch (e) {
            item.name = 'Invalid Profile';
          }
        } else if (event.kind === 10002) {
          item.relayCount = event.tags.filter(t => t[0] === 'r').length;
          item.name = `${item.relayCount} relays`;
        } else if (event.kind === 1) {
          item.summary = event.content.substring(0, 100);
          const topicTags = event.tags.filter(t => t[0] === 't');
          item.topics = topicTags.map(t => t[1]);
        } else if (event.kind === 7) {
          item.reaction = event.content;
          const eTags = event.tags.filter(t => t[0] === 'e');
          item.targetEvent = eTags[0]?.[1];
        }
        
        return item;
      });

      // Apply sorting
      items.sort((a, b) => {
        let compareA, compareB;
        
        if (sortBy === 'kind') {
          compareA = a.kind;
          compareB = b.kind;
        } else { // sortBy === 'created_at'
          compareA = a.created_at;
          compareB = b.created_at;
        }
        
        if (sortOrder === 'asc') {
          return compareA - compareB;
        } else {
          return compareB - compareA;
        }
      });
      
    } catch (error) {
      console.error('Failed to load preset data:', error);
    } finally {
      loading = false;
    }
  }
  
  function selectItem(item) {
    selectedItem = item;
    soundEffects.playClick();
  }
  
  // Sorting functions
  function changeSortBy(newSortBy) {
    if (sortBy === newSortBy) {
      // Toggle sort order if same field
      sortOrder = sortOrder === 'asc' ? 'desc' : 'asc';
    } else {
      sortBy = newSortBy;
      sortOrder = newSortBy === 'created_at' ? 'desc' : 'asc'; // Default desc for date, asc for kind
    }
    currentPage = 0; // Reset to first page
    soundEffects.playButton();
    loadPresetData();
  }
  
  function nextPage() {
    currentPage++;
    soundEffects.playClick();
    loadPresetData();
  }
  
  function prevPage() {
    if (currentPage > 0) {
      currentPage--;
      soundEffects.playClick();
      loadPresetData();
    }
  }
  
  // Transport controls
  function play() {
    if ($currentCassette) {
      isPlaying.set(true);
      soundEffects.playClick();
    }
  }
  
  function stop() {
    isPlaying.set(false);
    soundEffects.playClick();
  }
  
  onDestroy(() => {
    stopMeterAnimation();
  });
  
  // Watch for cassette changes to reload data
  $: if (selectedPreset && $cassettes.size > 0) {
    loadPresetData();
  }
</script>

<svelte:window bind:innerWidth bind:innerHeight />

<div class="studio-console">
  <!-- Compact header bar -->
  <div class="header-bar">
    <div class="logo">CASSETTE</div>
    <div class="status-display">
      <div class="power-led"></div>
      <span class="status-text">MULTI-DECK v2.1</span>
    </div>
    <div class="controls">
      <label class="sound-switch">
        <input type="checkbox" bind:checked={$soundEnabled} />
        <span>SND</span>
      </label>
    </div>
  </div>
  
  <!-- Main console -->
  <div class="console-grid">
    <!-- Left sidebar - Bank and Transport -->
    <div class="left-sidebar">
      <!-- Bank selector -->
      <div class="bank-section">
        <!-- <div class="section-label">BANK SELECT</div> -->
        <div class="bank-grid">
          {#each [1, 2, 3, 4, 5, 6, 7, 8, 9] as num}
            <button 
              class="bank-button"
              class:loaded={bankCassettes[num - 1]}
              class:active={currentBank === num}
              class:selected={bankCassettes[num - 1] && $selectedCassettes.has(bankCassettes[num - 1].id)}
              on:click={() => selectBank(num)}
            >
              <div class="button-number">{num}</div>
              {#if bankCassettes[num - 1]}
                <div class="cassette-indicator">
                  <div class="indicator-led"></div>
                  <div class="cassette-label">{bankCassettes[num - 1].name.substring(0, 6)}</div>
                </div>
                <button 
                  class="eject-button"
                  on:click|stopPropagation={() => ejectBank(num)}
                  title="Eject cassette"
                >
                  ⏏
                </button>
              {:else}
                <div class="empty-slot">EMPTY</div>
              {/if}
            </button>
          {/each}
        </div>
        
        <button class="load-button" on:click={() => fileInput.click()}>
          <span class="button-text">LOAD</span>
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
      
      <!-- Transport controls -->
      <div class="transport-section">
        <div class="section-label">TRANSPORT</div>
        <div class="transport-buttons">
          <button class="transport-btn record" disabled={$selectedCassettes.size === 0}>
            <span class="btn-icon">●</span>
          </button>
          <button 
            class="transport-btn play"
            class:active={$isPlaying}
            disabled={!$currentCassette}
            on:click={play}
          >
            <span class="btn-icon">▶</span>
          </button>
          <button 
            class="transport-btn stop"
            disabled={!$isPlaying}
            on:click={stop}
          >
            <span class="btn-icon">■</span>
          </button>
        </div>
      </div>
      
      <!-- VU meters -->
      <div class="meter-section">
        <div class="section-label">LEVELS</div>
        <div class="vu-meters">
          <div class="vu-meter">
            <div class="meter-scale">
              <div class="meter-bar" style="height: {leftMeterHeight}%"></div>
            </div>
            <div class="meter-label">L</div>
          </div>
          <div class="vu-meter">
            <div class="meter-scale">
              <div class="meter-bar" style="height: {rightMeterHeight}%"></div>
            </div>
            <div class="meter-label">R</div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- Center - Neural Analyzer -->
    <div class="center-area">
      <!-- Control Panel Header -->
      <div class="control-panel">
        <div class="panel-header">
          <div class="section-divider left">
            <div class="divider-label">PATTERN SELECT</div>
            <div class="control-row">
              {#each presets as preset}
                <button 
                  class="pattern-button"
                  class:active={selectedPreset?.id === preset.id}
                  on:click={() => selectPreset(preset)}
                >
                  <div class="led"></div>
                  <span class="label">{preset.label}</span>
                </button>
              {/each}
            </div>
          </div>
          
          <div class="section-divider right">
            <div class="divider-label">SORT ORDER</div>
            <div class="control-row">
              <button 
                class="sort-switch"
                class:active={sortBy === 'created_at'}
                on:click={() => changeSortBy('created_at')}
              >
                <div class="switch-indicator">
                  {#if sortBy === 'created_at'}
                    {sortOrder === 'desc' ? '↓' : '↑'}
                  {:else}
                    '-'
                  {/if}
                </div>
                <span class="switch-label">TIME</span>
              </button>
              
              <button 
                class="sort-switch"
                class:active={sortBy === 'kind'}
                on:click={() => changeSortBy('kind')}
              >
                <div class="switch-indicator">
                  {#if sortBy === 'kind'}
                    {sortOrder === 'desc' ? '↓' : '↑'}
                  {:else}
                    '-'
                  {/if}
                </div>
                <span class="switch-label">TYPE</span>
              </button>
            </div>
          </div>
        </div>
      </div>
      
      <!-- Data Display Matrix -->
      <div class="data-matrix">
        <div class="matrix-header">
          <div class="status-bar">
            <div class="status-indicator"></div>
            <span class="status-text">
              {#if selectedPreset}
                {selectedPreset.label} • {items.length} DETECTED
              {:else}
                NO PATTERN SELECTED
              {/if}
            </span>
            <div class="page-counter">
              <button 
                class="nav-button"
                disabled={currentPage === 0}
                on:click={prevPage}
              >◀</button>
              <span class="page-display">{String(currentPage + 1).padStart(2, '0')}</span>
              <button 
                class="nav-button"
                on:click={nextPage}
              >▶</button>
            </div>
          </div>
        </div>
        
        <div class="matrix-display">
          {#if loading}
            <div class="scan-indicator">
              <div class="scan-bar"></div>
              <span class="scan-text">SCANNING...</span>
            </div>
          {:else}
            <div class="data-cells">
              {#each Array(itemsPerPage) as _, i}
                {@const item = items[i]}
                <div 
                  class="data-cell"
                  class:active={item && selectedItem?.id === item.id}
                  class:occupied={item}
                  on:click={() => item && selectItem(item)}
                >
                  <div class="cell-header">
                    <span class="cell-index">{String(i + 1).padStart(2, '0')}</span>
                    <span class="cell-type">{item ? item.kind : '--'}</span>
                  </div>
                  <div class="cell-content">
                    {#if item}
                      {#if item.kind === 0}
                        <div class="data-snippet">{item.name || 'PROFILE'}</div>
                      {:else if item.kind === 10002}
                        <div class="data-snippet">{item.relayCount} RELAYS</div>
                      {:else if item.kind === 1}
                        <div class="data-snippet">{item.summary?.substring(0, 20) || 'NOTE'}...</div>
                      {:else if item.kind === 7}
                        <div class="data-snippet">{item.reaction || 'REACTION'}</div>
                      {:else}
                        <div class="data-snippet">EVENT {item.kind}</div>
                      {/if}
                    {:else}
                      <div class="cell-empty">EMPTY</div>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>
    
    <!-- Tape Deck Conveyor -->
    <div class="conveyor-area">
      <div class="conveyor-header">
        <span class="conveyor-title">MULTI-DECK</span>
        <span class="conveyor-version">v2.1</span>
      </div>
      
      <!-- Conveyor mechanism -->
      <div class="conveyor-mechanism">
        <div class="conveyor-track">
          <!-- Drive motor indicator -->
          <div class="motor-housing">
            <div class="motor {$isPlaying ? 'running' : ''}"></div>
            <div class="motor-label">DRIVE</div>
          </div>
          
          <!-- Conveyor belt with 9 tape slots -->
          <div class="conveyor-belt">
            <div class="belt-surface">
              {#each Array(9) as _, i}
                <div class="tape-slot" class:loaded={bankCassettes[i]} class:active={currentBank === i + 1}>
                  <div class="slot-number">{i + 1}</div>
                  
                  {#if bankCassettes[i]}
                    <!-- Loaded cassette representation -->
                    <div class="cassette-body">
                      <div class="cassette-face">
                        <div class="cassette-window"></div>
                        <div class="cassette-reels">
                          <div class="reel left {$isPlaying && currentBank === i + 1 ? 'spinning' : ''}"></div>
                          <div class="reel right {$isPlaying && currentBank === i + 1 ? 'spinning' : ''}"></div>
                        </div>
                      </div>
                      <div class="cassette-label-strip">
                        <div class="cassette-name">{bankCassettes[i].name.substring(0, 8)}</div>
                      </div>
                      
                      <!-- Eject mechanism -->
                      <button 
                        class="eject-lever"
                        on:click={() => ejectBank(i + 1)}
                        title="Eject cassette {i + 1}"
                      >
                        <div class="lever-handle"></div>
                      </button>
                    </div>
                  {:else}
                    <!-- Empty slot -->
                    <div class="empty-slot-mechanism">
                      <div class="slot-guides">
                        <div class="guide left"></div>
                        <div class="guide right"></div>
                      </div>
                      <div class="empty-indicator">EMPTY</div>
                    </div>
                  {/if}
                  
                  <!-- Selection indicator -->
                  <div class="selection-led {bankCassettes[i] && $selectedCassettes.has(bankCassettes[i].id) ? 'selected' : ''}"></div>
                  
                  <!-- Click handler for selection -->
                  <button 
                    class="slot-selector"
                    on:click={() => selectBank(i + 1)}
                    disabled={!bankCassettes[i]}
                  ></button>
                </div>
              {/each}
            </div>
            
            <!-- Belt texture/pattern -->
            <div class="belt-texture"></div>
          </div>
          
          <!-- Conveyor controls -->
          <div class="conveyor-controls">
            <div class="control-group">
              <button class="conveyor-btn load" on:click={() => fileInput.click()}>
                <span class="btn-symbol">↑</span>
                <span class="btn-text">LOAD</span>
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
            
            <div class="control-group">
              <div class="speed-indicator">
                <div class="speed-led {$isPlaying ? 'active' : ''}"></div>
                <div class="speed-label">MOTOR</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- Right sidebar - Hardware Detail Panel -->
    <div class="detail-area">
      <!-- Physical unit label -->
      <div class="unit-label">
        <div class="unit-name">NEURAL ANALYZER</div>
        <div class="unit-model">MODEL NA-2000</div>
      </div>
      
      <!-- Status LEDs -->
      <div class="status-leds">
        <div class="led-group">
          <div class="led {selectedItem ? 'active' : ''}" data-label="DATA"></div>
          <div class="led-label">DATA</div>
        </div>
        <div class="led-group">
          <div class="led {$isPlaying ? 'active' : ''}" data-label="PROC"></div>
          <div class="led-label">PROC</div>
        </div>
        <div class="led-group">
          <div class="led active" data-label="PWR"></div>
          <div class="led-label">PWR</div>
        </div>
      </div>
      
      <!-- Main CRT display -->
      <div class="crt-display">
        <div class="crt-bezel">
          <div class="crt-screen">
            <div class="scan-lines"></div>
            <div class="crt-content">
              {#if selectedItem}
                <div class="crt-header">
                  <span class="terminal-prompt">NEURAL_SYS:DATA_$</span>
                  <span class="cursor blink">_</span>
                </div>
                
                <div class="data-readout">
                  <div class="data-line">
                    <span class="data-label">ID:</span>
                    <span class="data-value">{selectedItem.id.substring(0, 24)}</span>
                  </div>
                  <div class="data-line">
                    <span class="data-label">KIND:</span>
                    <span class="data-value">{selectedItem.kind}</span>
                  </div>
                  <div class="data-line">
                    <span class="data-label">TIME:</span>
                    <span class="data-value">{new Date(selectedItem.created_at * 1000).toLocaleString()}</span>
                  </div>
                  
                  {#if selectedItem.kind === 0}
                    <div class="data-line">
                      <span class="data-label">NAME:</span>
                      <span class="data-value">{selectedItem.name}</span>
                    </div>
                    {#if selectedItem.about}
                      <div class="data-block">
                        <div class="data-label">PROFILE:</div>
                        <div class="data-text">{selectedItem.about.substring(0, 200)}</div>
                      </div>
                    {/if}
                  {:else if selectedItem.kind === 1}
                    <div class="data-block">
                      <div class="data-label">CONTENT:</div>
                      <div class="data-text">{selectedItem.content.substring(0, 300)}</div>
                    </div>
                  {:else if selectedItem.kind === 7}
                    <div class="data-line">
                      <span class="data-label">REACTION:</span>
                      <span class="data-value reaction-display">{selectedItem.reaction}</span>
                    </div>
                  {/if}
                </div>
              {:else}
                <div class="crt-standby">
                  <div class="standby-text">
                    <div class="scan-wave"></div>
                    <div>NEURAL ANALYZER STANDBY</div>
                    <div class="standby-sub">SELECT DATA FOR ANALYSIS</div>
                  </div>
                </div>
              {/if}
            </div>
          </div>
        </div>
        
        <!-- Physical control knobs -->
        <div class="crt-controls">
          <div class="knob-group">
            <div class="knob brightness"></div>
            <div class="knob-label">BRIGHT</div>
          </div>
          <div class="knob-group">
            <div class="knob contrast"></div>
            <div class="knob-label">CONT</div>
          </div>
          <div class="knob-group">
            <div class="knob vertical"></div>
            <div class="knob-label">V-SYNC</div>
          </div>
        </div>
      </div>
      
      <!-- Physical switches -->
      <div class="hardware-switches">
        <div class="switch-group">
          <div class="toggle-switch">
            <div class="switch-lever"></div>
          </div>
          <div class="switch-label">FILTER</div>
        </div>
        <div class="switch-group">
          <div class="toggle-switch active">
            <div class="switch-lever"></div>
          </div>
          <div class="switch-label">NORM</div>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  /* Hardware controls should not have text selection */
  button, .unit-label, .section-label, .logo, .slot-number {
    user-select: none;
    -webkit-user-select: none;
    -moz-user-select: none;
    -ms-user-select: none;
  }
  
  .studio-console {
    width: 100%;
    height: 100%;
    max-width: none;
    background: 
      linear-gradient(135deg, #4a4a4a 0%, #3a3a3a 50%, #2a2a2a 100%);
    border-radius: clamp(4px, 1vw, 8px);
    box-shadow: 
      0 15px 50px rgba(0, 0, 0, 0.7),
      inset 0 2px 0 rgba(255, 255, 255, 0.15),
      inset 0 -2px 0 rgba(0, 0, 0, 0.2);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: 2px solid #555;
  }
  
  /* Compact header bar */
  .header-bar {
    height: 24px;
    background: #0a0a0a;
    border-bottom: 1px solid #222;
    display: flex;
    align-items: center;
    padding: 0 6px;
    font-size: 8px;
    flex-shrink: 0;
  }
  
  .logo {
    font-weight: bold;
    color: #aaa;
    letter-spacing: 0.1em;
    font-size: 9px;
  }
  
  .status-display {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
  }
  
  .power-led {
    width: 3px;
    height: 3px;
    background: #0f0;
  }
  
  .status-text {
    color: #666;
    font-size: 7px;
    letter-spacing: 0.2em;
  }
  
  .controls {
    display: flex;
    align-items: center;
  }
  
  .sound-switch {
    display: flex;
    align-items: center;
    gap: 2px;
    cursor: pointer;
    font-size: 6px;
    color: #666;
  }
  
  .sound-switch input[type="checkbox"] {
    width: 20px;
    height: 10px;
    appearance: none;
    background: #1a1a1a;
    border: 1px solid #333;
    position: relative;
    cursor: pointer;
  }
  
  .sound-switch input[type="checkbox"]::after {
    content: '';
    position: absolute;
    width: 8px;
    height: 8px;
    background: #444;
    top: 0;
    left: 0;
    transition: all 0.1s;
  }
  
  .sound-switch input[type="checkbox"]:checked {
    background: #0a2a0a;
  }
  
  .sound-switch input[type="checkbox"]:checked::after {
    left: 10px;
    background: #0f0;
  }
  
  /* Console grid */
  .console-grid {
    display: grid;
    background: 
      linear-gradient(90deg, #2a2a2a 0%, #252525 50%, #2a2a2a 100%);
    flex: 1;
    overflow: hidden;
    gap: 4px; /* Module spacing */
    padding: 4px;
    
    /* Desktop layout */
    grid-template-columns: minmax(200px, 20vw) 1fr minmax(250px, 25vw);
    grid-template-rows: minmax(80px, 15vh) 1fr;
    grid-template-areas: 
      "conveyor conveyor conveyor"
      "sidebar center detail";
  }
  
  /* Responsive breakpoints */
  @media (max-width: 1024px) and (orientation: landscape) {
    .console-grid {
      grid-template-columns: minmax(180px, 25vw) 1fr minmax(200px, 30vw);
    }
  }
  
  @media (max-width: 768px) {
    .console-grid {
      grid-template-columns: 1fr;
      grid-template-rows: minmax(100px, 20vh) 1fr auto auto;
      grid-template-areas: 
        "conveyor"
        "center"
        "detail"
        "sidebar";
    }
  }
  
  @media (max-width: 480px) {
    .console-grid {
      grid-template-areas: 
        "center"
        "detail"
        "sidebar";
    }
  }
  
  /* Portrait orientation optimizations */
  @media (orientation: portrait) and (max-height: 900px) {
    .console-grid {
      grid-template-columns: 1fr;
      grid-template-rows: 1fr auto auto;
      grid-template-areas: 
        "center"
        "detail"
        "sidebar";
    }
  }
  
  /* Very small screens */
  @media (max-width: 320px) {
    .console-grid {
      grid-template-rows: 1fr minmax(150px, 30vh) auto;
    }
  }
  
  /* Left sidebar - Eurorack Bank Module */
  .left-sidebar {
    grid-area: sidebar;
    background: 
      linear-gradient(180deg, #2a2a2a 0%, #1a1a1a 50%, #151515 100%);
    padding: clamp(10px, 2vw, 20px);
    display: flex;
    flex-direction: column;
    gap: clamp(10px, 2vh, 20px);
    border: 2px solid #444;
    border-radius: 4px;
    overflow-y: auto;
    position: relative;
    box-shadow: 
      inset 0 2px 0 rgba(255, 255, 255, 0.1),
      inset 0 -2px 0 rgba(0, 0, 0, 0.3),
      0 2px 4px rgba(0, 0, 0, 0.2);
  }
  
  /* Eurorack mounting holes */
  .left-sidebar::before,
  .left-sidebar::after {
    content: '';
    position: absolute;
    right: 8px;
    width: 6px;
    height: 6px;
    background: #0a0a0a;
    border-radius: 50%;
    border: 1px solid #333;
    box-shadow: 
      inset 0 1px 2px rgba(0, 0, 0, 0.8),
      0 1px 0 rgba(255, 255, 255, 0.1);
  }
  
  .left-sidebar::before {
    top: 12px;
  }
  
  .left-sidebar::after {
    bottom: 12px;
  }
  
  @media (max-width: 768px) {
    .left-sidebar {
      flex-direction: row;
      border-right: none;
      border-top: 2px solid #333;
      padding: clamp(8px, 2vw, 15px);
      gap: clamp(8px, 2vw, 15px);
      overflow-x: auto;
      overflow-y: hidden;
    }
  }
  
  .section-label {
    font-size: clamp(8px, 1.5vw, 12px);
    color: #888;
    letter-spacing: 0.1em;
    margin-bottom: clamp(5px, 1vh, 10px);
    text-align: center;
  }
  
  .bank-section {
    flex: 1;
    min-width: 150px;
    position: relative;
    background: 
      linear-gradient(180deg, rgba(255,255,255,0.03) 0%, transparent 100%);
    border-radius: 6px;
    border: 1px solid #333;
    padding: 12px;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.4);
  }
  
  /* Bank section module label */
  .bank-section::before {
    content: 'BANK MODULE';
    position: absolute;
    top: -8px;
    left: 12px;
    background: #1a1a1a;
    padding: 2px 8px;
    font-size: 6px;
    color: #888;
    letter-spacing: 0.5px;
    font-weight: bold;
    border: 1px solid #333;
    border-radius: 2px;
  }
  
  .bank-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px; /* Fixed physical spacing */
    margin-bottom: 18px;
    padding: 8px;
    background: #0a0a0a;
    border-radius: 4px;
    border: 1px solid #333;
    box-shadow: inset 0 3px 6px rgba(0, 0, 0, 0.8);
  }
  
  @media (max-width: 768px) {
    .bank-section {
      min-width: 140px;
    }
    .bank-grid {
      gap: 10px;
    }
  }
  
  @media (max-width: 480px) {
    .bank-grid {
      grid-template-columns: repeat(2, 1fr);
      gap: 8px;
    }
  }
  
  .bank-button {
    width: 100%;
    height: 58px; /* Fixed physical height */
    background: 
      linear-gradient(145deg, #4a4a4a 0%, #3a3a3a 45%, #2a2a2a 100%);
    border: 2px solid #666;
    border-radius: 6px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    padding: 4px;
    transition: all 0.15s ease-out;
    position: relative;
    overflow: hidden;
    font-size: 8px; /* Fixed small text like real hardware */
    font-weight: bold;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    touch-action: manipulation;
    box-shadow: 
      0 4px 8px rgba(0, 0, 0, 0.4),
      inset 0 1px 0 rgba(255, 255, 255, 0.2),
      inset 0 -1px 0 rgba(0, 0, 0, 0.3);
  }
  
  @media (hover: hover) {
    .bank-button:hover:not(:active) {
      background: 
        linear-gradient(145deg, #5a5a5a 0%, #4a4a4a 45%, #3a3a3a 100%);
      transform: translateY(-1px);
      box-shadow: 
        0 6px 12px rgba(0, 0, 0, 0.3),
        inset 0 1px 0 rgba(255, 255, 255, 0.25);
    }
  }
  
  .bank-button:active {
    transform: translateY(2px);
    box-shadow: 
      0 2px 4px rgba(0, 0, 0, 0.4),
      inset 0 2px 4px rgba(0, 0, 0, 0.4);
  }
  
  .bank-button:hover {
    background: linear-gradient(135deg, #4a4a4a 0%, #3a3a3a 100%);
  }
  
  .bank-button.active {
    border-color: #0f0;
    box-shadow: 0 0 8px rgba(0, 255, 0, 0.3);
  }
  
  .bank-button.selected {
    background: linear-gradient(135deg, #1a3a1a 0%, #0a2a0a 100%);
  }
  
  .button-number {
    position: absolute;
    top: 2px;
    left: 3px;
    font-size: 10px;
    font-weight: bold;
    color: #999;
    text-shadow: 0 1px 1px rgba(0, 0, 0, 0.8);
  }
  
  .cassette-indicator {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
  }
  
  .indicator-led {
    width: 4px;
    height: 4px;
    background: #222;
    border-radius: 50%;
    border: 1px solid #444;
    box-shadow: inset 0 1px 1px rgba(0, 0, 0, 0.8);
  }
  
  .bank-button.active .indicator-led {
    background: radial-gradient(circle, #0f0 40%, #0a0 100%);
    box-shadow: 
      0 0 4px #0f0,
      0 0 8px rgba(0, 255, 0, 0.4),
      inset 0 1px 1px rgba(255, 255, 255, 0.3);
  }
  
  .cassette-label {
    font-size: 6px; /* Very small like real hardware labels */
    color: #bbb;
    text-align: center;
    width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-shadow: 0 1px 1px rgba(0, 0, 0, 0.8);
    max-width: 45px; /* Physical constraint */
  }
  
  .empty-slot {
    font-size: 7px;
    color: #444;
    text-shadow: 0 1px 1px rgba(0, 0, 0, 0.8);
  }
  
  .eject-button {
    position: absolute;
    bottom: 1px;
    right: 1px;
    width: 12px;
    height: 12px;
    background: 
      linear-gradient(145deg, #444 0%, #222 100%);
    border: 1px solid #555;
    border-radius: 2px;
    color: #aaa;
    font-size: 6px;
    cursor: pointer;
    opacity: 0;
    transition: all 0.2s;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 
      0 1px 2px rgba(0, 0, 0, 0.6),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
  }
  
  .bank-button:hover .eject-button {
    opacity: 0.8;
  }
  
  .eject-button:hover {
    opacity: 1 !important;
    background: 
      linear-gradient(145deg, #555 0%, #333 100%);
    transform: translateY(-0.5px);
  }
  
  .load-button {
    width: 100%;
    height: 35px;
    background: linear-gradient(to bottom, #4a4a4a, #3a3a3a);
    border: 1px solid #555;
    border-radius: 4px;
    color: #ccc;
    font-size: 11px;
    letter-spacing: 0.1em;
    cursor: pointer;
    transition: all 0.1s;
  }
  
  .load-button:hover {
    background: linear-gradient(to bottom, #5a5a5a, #4a4a4a);
  }
  
  .transport-section {
    border-top: 2px solid #333;
    padding-top: 15px;
    position: relative;
    background: 
      linear-gradient(180deg, rgba(255,255,255,0.02) 0%, transparent 100%);
    border-radius: 6px;
    border: 1px solid #333;
    padding: 12px;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.4);
  }
  
  /* Transport section module label */
  .transport-section::before {
    content: 'TRANSPORT';
    position: absolute;
    top: -8px;
    left: 12px;
    background: #1a1a1a;
    padding: 2px 8px;
    font-size: 6px;
    color: #888;
    letter-spacing: 0.5px;
    font-weight: bold;
    border: 1px solid #333;
    border-radius: 2px;
  }
  
  .transport-buttons {
    display: flex;
    justify-content: center;
    gap: 8px;
  }
  
  .transport-btn {
    width: 40px;
    height: 40px;
    background: linear-gradient(to bottom, #4a4a4a, #3a3a3a);
    border: 2px solid #555;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.1s;
  }
  
  .transport-btn:hover:not(:disabled) {
    background: linear-gradient(to bottom, #5a5a5a, #4a4a4a);
  }
  
  .transport-btn:active:not(:disabled) {
    transform: translateY(1px);
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.5);
  }
  
  .transport-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
  
  .transport-btn.active {
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.5);
    transform: translateY(1px);
  }
  
  .transport-btn.record .btn-icon {
    color: #f00;
  }
  
  .transport-btn.play .btn-icon {
    color: #0f0;
  }
  
  .btn-icon {
    font-size: 16px;
    color: #ccc;
  }
  
  .meter-section {
    border-top: 2px solid #333;
    padding-top: 15px;
    position: relative;
    background: 
      linear-gradient(180deg, rgba(0,255,0,0.02) 0%, transparent 100%);
    border-radius: 6px;
    border: 1px solid #333;
    padding: 12px;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.4);
  }
  
  /* Meter section module label */
  .meter-section::before {
    content: 'VU METERS';
    position: absolute;
    top: -8px;
    left: 12px;
    background: #1a1a1a;
    padding: 2px 8px;
    font-size: 6px;
    color: #0a0;
    letter-spacing: 0.5px;
    font-weight: bold;
    border: 1px solid #333;
    border-radius: 2px;
    text-shadow: 0 0 3px #0a0;
  }
  
  .vu-meters {
    display: flex;
    justify-content: center;
    gap: 15px;
  }
  
  .vu-meter {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
  }
  
  .meter-scale {
    width: 20px;
    height: 80px;
    background: #0a0a0a;
    border: 1px solid #333;
    border-radius: 2px;
    position: relative;
    overflow: hidden;
  }
  
  .meter-bar {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    background: linear-gradient(to top, #0a0 0%, #0f0 70%, #ff0 85%, #f00 95%);
    transition: height 0.05s ease-out;
  }
  
  .meter-label {
    font-size: 9px;
    color: #666;
  }
  
  /* Center area - Neural Analyzer Module */
  .center-area {
    grid-area: center;
    display: flex;
    flex-direction: column;
    background: #0a0a0a;
    border: 1px solid #333;
    position: relative;
    overflow: hidden;
  }
  
  /* Control Panel */
  .control-panel {
    background: #0f0f0f;
    border-bottom: 1px solid #222;
    padding: 4px 6px;
  }
  
  .panel-header {
    display: flex;
    gap: 0;
  }
  
  .section-divider {
    flex: 1;
    background: #1a1a1a;
    padding: 4px 6px;
  }
  
  .section-divider.left {
    border-right: 1px solid #0a0a0a;
  }
  
  .section-divider.right {
    border-left: 1px solid #2a2a2a;
  }
  
  .divider-label {
    font-size: 6px;
    color: #666;
    text-align: center;
    margin-bottom: 2px;
    letter-spacing: 0.3px;
    text-transform: uppercase;
  }
  
  .control-row {
    display: flex;
    gap: 2px;
    justify-content: center;
  }
  
  /* Pattern Select Buttons */
  .pattern-button {
    height: 18px;
    min-width: 40px;
    background: #2a2a2a;
    border: none;
    color: #888;
    font-size: 6px;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 4px;
    cursor: pointer;
    transition: all 0.05s;
    position: relative;
  }
  
  .pattern-button::after {
    content: '';
    position: absolute;
    inset: 0;
    border: 1px solid #1a1a1a;
    box-shadow: inset 0 1px 0 #333;
  }
  
  .pattern-button:hover {
    background: #333;
  }
  
  .pattern-button.active {
    background: #0a3a0a;
    color: #0f0;
  }
  
  .pattern-button.active::after {
    border-color: #0a0;
    box-shadow: inset 0 1px 0 #0a0;
  }
  
  .pattern-button .led {
    width: 4px;
    height: 4px;
    background: #222;
    flex-shrink: 0;
  }
  
  .pattern-button.active .led {
    background: #0f0;
  }
  
  .pattern-button .label {
    letter-spacing: 0.3px;
    font-weight: bold;
  }
  
  /* Sort Switch Buttons */
  .sort-switch {
    height: 18px;
    width: 32px;
    background: #2a2a2a;
    border: none;
    color: #888;
    font-size: 6px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.05s;
    position: relative;
  }
  
  .sort-switch::after {
    content: '';
    position: absolute;
    inset: 0;
    border: 1px solid #1a1a1a;
    box-shadow: inset 0 1px 0 #333;
  }
  
  .sort-switch:hover {
    background: #333;
  }
  
  .sort-switch.active {
    background: #0a3a0a;
    color: #0f0;
  }
  
  .sort-switch.active::after {
    border-color: #0a0;
    box-shadow: inset 0 1px 0 #0a0;
  }
  
  .switch-indicator {
    font-size: 8px;
    line-height: 1;
    margin-bottom: 2px;
    font-weight: bold;
  }
  
  .switch-label {
    font-size: 6px;
    letter-spacing: 0.3px;
    font-weight: bold;
  }
  
  /* Data Matrix */
  .data-matrix {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: #000;
  }
  
  .matrix-header {
    border-bottom: 1px solid #333;
  }
  
  .status-bar {
    display: flex;
    align-items: center;
    padding: 2px 4px;
    background: #0f0f0f;
    gap: 4px;
    font-size: 6px;
    color: #0a0;
    height: 20px;
  }
  
  .status-indicator {
    width: 4px;
    height: 4px;
    background: #0f0;
    flex-shrink: 0;
  }
  
  .status-text {
    flex: 1;
    letter-spacing: 0.3px;
  }
  
  .page-counter {
    display: flex;
    align-items: center;
    gap: 3px;
  }
  
  .nav-button {
    width: 14px;
    height: 14px;
    background: #1a1a1a;
    border: none;
    color: #666;
    font-size: 6px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  
  .nav-button:hover:not(:disabled) {
    background: #2a2a2a;
    color: #aaa;
  }
  
  .nav-button:disabled {
    opacity: 0.2;
    cursor: default;
  }
  
  .page-display {
    font-size: 8px;
    color: #0f0;
    font-family: monospace;
    margin: 0 3px;
    font-weight: bold;
  }
  
  /* Matrix Display */
  .matrix-display {
    flex: 1;
    padding: 2px;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex;
    flex-direction: column;
  }
  
  .scan-indicator {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 12px;
  }
  
  .scan-bar {
    width: 80px;
    height: 3px;
    background: linear-gradient(90deg, transparent 0%, #0f0 50%, transparent 100%);
    animation: scan 1.5s infinite;
  }
  
  @keyframes scan {
    0% { transform: translateX(-40px); opacity: 0; }
    50% { opacity: 1; }
    100% { transform: translateX(40px); opacity: 0; }
  }
  
  .scan-text {
    font-size: 8px;
    color: #0a0;
    letter-spacing: 0.5px;
    font-weight: bold;
  }
  
  .data-cells {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(90px, 1fr));
    gap: 1px;
    background: #0a0a0a;
    flex: 1;
    align-content: start;
  }
  
  .data-cell {
    background: #111;
    height: 28px;
    display: flex;
    flex-direction: column;
    cursor: pointer;
    transition: all 0.05s;
    overflow: hidden;
    position: relative;
  }
  
  .data-cell:hover.occupied {
    background: #1a1a1a;
  }
  
  .data-cell.active {
    background: #0a2a0a;
  }
  
  .cell-header {
    display: flex;
    justify-content: space-between;
    padding: 1px 2px;
    background: transparent;
    font-size: 5px;
    color: #444;
    flex-shrink: 0;
  }
  
  .data-cell.occupied .cell-header {
    color: #0a0;
  }
  
  .cell-index {
    font-family: monospace;
    font-weight: bold;
  }
  
  .cell-type {
    font-family: monospace;
    color: #888;
    font-weight: bold;
  }
  
  .data-cell.occupied .cell-type {
    color: #0f0;
  }
  
  .cell-content {
    flex: 1;
    padding: 3px;
    display: flex;
    align-items: center;
    font-size: 6px;
    overflow: hidden;
  }
  
  .data-snippet {
    color: #0f0;
    line-height: 1.2;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 100%;
    font-weight: bold;
  }
  
  .cell-empty {
    color: #333;
    font-size: 6px;
    text-align: center;
    width: 100%;
    font-weight: bold;
    letter-spacing: 0.2px;
  }
  
  .preset-section {
    padding: clamp(10px, 2vw, 20px);
    border-bottom: 2px solid #333;
    flex-shrink: 0;
    position: relative;
    background: 
      linear-gradient(180deg, rgba(255,255,255,0.02) 0%, transparent 100%);
  }
  
  /* Add module divider line with screws */
  .preset-section::after {
    content: '';
    position: absolute;
    bottom: -2px;
    left: 20px;
    right: 20px;
    height: 2px;
    background: 
      linear-gradient(90deg, 
        #333 0%, 
        #444 20%, 
        #333 40%, 
        #222 50%, 
        #333 60%, 
        #444 80%, 
        #333 100%
      );
    box-shadow: 0 1px 0 rgba(255, 255, 255, 0.1);
  }
  
  .preset-buttons {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 15px; /* Fixed physical spacing */
    padding: 12px;
    background: #0a0a0a;
    border-radius: 6px;
    border: 2px solid #333;
    box-shadow: inset 0 4px 8px rgba(0, 0, 0, 0.8);
  }
  
  @media (max-width: 768px) {
    .preset-buttons {
      grid-template-columns: repeat(3, 1fr);
      gap: 12px;
    }
  }
  
  @media (max-width: 480px) {
    .preset-buttons {
      grid-template-columns: repeat(2, 1fr);
      gap: 10px;
    }
  }
  
  .preset-button {
    height: 48px; /* Fixed physical height */
    background: 
      linear-gradient(145deg, #4a4a4a 0%, #3a3a3a 45%, #2a2a2a 100%);
    border: 2px solid #666;
    border-radius: 8px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 3px;
    transition: all 0.15s ease-out;
    touch-action: manipulation;
    position: relative;
    overflow: hidden;
    box-shadow: 
      0 4px 8px rgba(0, 0, 0, 0.4),
      inset 0 1px 0 rgba(255, 255, 255, 0.2),
      inset 0 -1px 0 rgba(0, 0, 0, 0.3);
  }
  
  @media (hover: hover) {
    .preset-button:hover:not(:active) {
      background: 
        linear-gradient(145deg, #5a5a5a 0%, #4a4a4a 45%, #3a3a3a 100%);
      transform: translateY(-1px);
      box-shadow: 
        0 6px 12px rgba(0, 0, 0, 0.3),
        inset 0 1px 0 rgba(255, 255, 255, 0.25);
    }
  }
  
  .preset-button:active {
    transform: translateY(2px);
    box-shadow: 
      0 2px 4px rgba(0, 0, 0, 0.4),
      inset 0 2px 4px rgba(0, 0, 0, 0.4);
  }
  
  .preset-button.active {
    background: 
      linear-gradient(145deg, #2a4a2a 0%, #1a3a1a 45%, #0a2a0a 100%);
    border-color: #0a0;
    box-shadow: 
      0 0 12px rgba(0, 255, 0, 0.4),
      0 4px 8px rgba(0, 0, 0, 0.4),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
  }
  
  .preset-button.active .button-led {
    background: radial-gradient(circle, #0f0 40%, #0a0 100%);
    box-shadow: 
      0 0 6px #0f0,
      0 0 12px rgba(0, 255, 0, 0.5);
  }
  
  .button-led {
    width: 5px;
    height: 5px;
    background: #222;
    border-radius: 50%;
    border: 1px solid #444;
    transition: all 0.2s;
    box-shadow: inset 0 1px 1px rgba(0, 0, 0, 0.8);
  }
  
  .button-label {
    font-size: 7px; /* Very small hardware text */
    color: #bbb;
    letter-spacing: 0.5px;
    text-transform: uppercase;
    font-weight: bold;
    text-shadow: 0 1px 1px rgba(0, 0, 0, 0.8);
    text-align: center;
    max-width: 80px; /* Physical constraint */
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Sort controls */
  .sort-section {
    padding: clamp(8px, 1.5vw, 12px);
    background: #1a1a1a;
    border: 1px solid #333;
    border-radius: 4px;
    margin-bottom: clamp(8px, 1.5vw, 15px);
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.8);
  }

  .sort-label {
    font-size: 8px;
    color: #888;
    letter-spacing: 0.1em;
    margin-bottom: 6px;
    text-align: center;
  }

  .sort-controls {
    display: flex;
    gap: 8px;
    justify-content: center;
  }

  .sort-button {
    width: 60px;
    height: 32px;
    background: linear-gradient(145deg, #3a3a3a 0%, #2a2a2a 45%, #1a1a1a 100%);
    border: 1px solid #555;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    transition: all 0.1s;
    box-shadow: 
      0 2px 4px rgba(0, 0, 0, 0.3),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
  }

  .sort-button:hover {
    background: linear-gradient(145deg, #4a4a4a 0%, #3a3a3a 45%, #2a2a2a 100%);
    transform: translateY(-1px);
  }

  .sort-button:active {
    transform: translateY(1px);
    box-shadow: 
      0 1px 2px rgba(0, 0, 0, 0.3),
      inset 0 1px 3px rgba(0, 0, 0, 0.4);
  }

  .sort-button.active {
    background: linear-gradient(145deg, #2a3a2a 0%, #1a2a1a 45%, #0a1a0a 100%);
    border-color: #0a0;
    box-shadow: 
      0 0 8px rgba(0, 255, 0, 0.3),
      0 2px 4px rgba(0, 0, 0, 0.4),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
  }

  .sort-indicator {
    font-size: 10px;
    color: #aaa;
    font-weight: bold;
    min-height: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .sort-button.active .sort-indicator {
    color: #0f0;
    text-shadow: 0 0 3px #0f0;
  }

  .sort-text {
    font-size: 6px;
    color: #888;
    letter-spacing: 0.5px;
    font-weight: bold;
  }

  .sort-button.active .sort-text {
    color: #0a0;
  }
  
  .data-section {
    flex: 1;
    padding: clamp(10px, 2vw, 20px);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  
  .data-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 15px;
    padding-bottom: 10px;
    border-bottom: 1px solid #444;
  }
  
  .preset-name {
    font-size: 12px;
    color: #0f0;
    letter-spacing: 0.1em;
  }
  
  .item-count {
    font-size: 10px;
    color: #888;
  }
  
  .no-preset {
    font-size: 10px;
    color: #666;
  }
  
  .pagination {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  
  .page-button {
    width: 25px;
    height: 25px;
    background: linear-gradient(to bottom, #3a3a3a, #2a2a2a);
    border: 1px solid #444;
    border-radius: 3px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    color: #0f0;
  }
  
  .page-button:hover:not(:disabled) {
    background: linear-gradient(to bottom, #4a4a4a, #3a3a3a);
  }
  
  .page-button:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
  
  .page-info {
    font-size: 9px;
    color: #888;
    min-width: 50px;
    text-align: center;
  }
  
  .data-grid {
    display: grid;
    gap: clamp(3px, 0.5vw, 8px);
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    background: 
      radial-gradient(circle at 20% 20%, rgba(255,255,255,0.02) 0%, transparent 40%),
      #0a0a0a;
    border-radius: 6px;
    border: 2px solid #333;
    box-shadow: inset 0 4px 8px rgba(0, 0, 0, 0.6);
    position: relative;
    
    /* Dynamic grid based on container width */
    grid-template-columns: repeat(auto-fill, minmax(clamp(60px, 12vw, 120px), 1fr));
  }
  
  /* Add screen holes to data grid */
  .data-grid::before,
  .data-grid::after {
    content: '';
    position: absolute;
    top: 8px;
    width: 4px;
    height: 4px;
    background: #000;
    border-radius: 50%;
    border: 1px solid #222;
    box-shadow: 
      inset 0 1px 1px rgba(0, 0, 0, 0.9),
      0 1px 0 rgba(255, 255, 255, 0.05);
    z-index: 10;
  }
  
  .data-grid::before {
    right: 20px;
  }
  
  .data-grid::after {
    right: 8px;
  }
  
  /* Ensure minimum number of columns on larger screens */
  @media (min-width: 1200px) {
    .data-grid {
      grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
    }
  }
  
  @media (max-width: 768px) {
    .data-grid {
      grid-template-columns: repeat(auto-fill, minmax(80px, 1fr));
    }
  }
  
  @media (max-width: 480px) {
    .data-grid {
      grid-template-columns: repeat(auto-fill, minmax(70px, 1fr));
    }
  }
  
  @media (max-width: 320px) {
    .data-grid {
      grid-template-columns: repeat(auto-fill, minmax(60px, 1fr));
    }
  }
  
  .data-button {
    aspect-ratio: 1;
    background: linear-gradient(to bottom, #3a3a3a, #2a2a2a);
    border: clamp(1px, 0.1vw, 2px) solid #444;
    border-radius: clamp(2px, 0.3vw, 4px);
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: clamp(1px, 0.2vh, 3px);
    padding: clamp(2px, 0.5vw, 6px);
    transition: all 0.1s;
    overflow: hidden;
    touch-action: manipulation;
    min-height: 40px;
  }
  
  .data-button:hover:not(.empty) {
    background: linear-gradient(to bottom, #4a4a4a, #3a3a3a);
    border-color: #0a0;
  }
  
  .data-button.selected {
    border-color: #0f0;
    box-shadow: 0 0 4px rgba(0, 255, 0, 0.3);
  }
  
  .data-button.empty {
    opacity: 0.3;
    cursor: default;
  }
  
  .loading {
    grid-column: 1 / -1;
    text-align: center;
    color: #0f0;
    padding: 40px;
    font-size: 12px;
  }
  
  .profile-pic {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    object-fit: cover;
  }
  
  .profile-placeholder {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: #0f0;
    color: #000;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: bold;
  }
  
  .kind-indicator {
    position: absolute;
    top: 2px;
    left: 2px;
    font-size: 6px;
    color: #0a0;
    background: rgba(0, 0, 0, 0.8);
    padding: 1px 3px;
    border-radius: 2px;
    font-weight: bold;
    z-index: 2;
  }

  .item-label {
    font-size: 8px;
    color: #ccc;
    text-align: center;
    width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  
  .note-preview {
    font-size: 7px;
    color: #aaa;
    text-align: left;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    width: 100%;
  }
  
  .relay-icon {
    font-size: 16px;
  }
  
  .reaction-emoji {
    font-size: 18px;
  }
  
  .empty-label {
    font-size: 8px;
    color: #555;
  }
  
  /* Detail area - Analyzer Module */
  .detail-area {
    grid-area: detail;
    background: 
      linear-gradient(135deg, #2a2a2a 0%, #1a1a1a 50%, #151515 100%);
    border: 2px solid #444;
    border-radius: 4px;
    display: flex;
    flex-direction: column;
    padding: clamp(15px, 3vw, 25px);
    gap: clamp(10px, 2vh, 20px);
    overflow: hidden;
    position: relative;
    box-shadow: 
      inset 0 2px 0 rgba(255, 255, 255, 0.1),
      inset 0 -2px 0 rgba(0, 0, 0, 0.3),
      0 2px 4px rgba(0, 0, 0, 0.2);
  }
  
  /* Detail module mounting holes */
  .detail-area::before,
  .detail-area::after {
    content: '';
    position: absolute;
    left: 8px;
    width: 6px;
    height: 6px;
    background: #0a0a0a;
    border-radius: 50%;
    border: 1px solid #333;
    box-shadow: 
      inset 0 1px 2px rgba(0, 0, 0, 0.8),
      0 1px 0 rgba(255, 255, 255, 0.1);
    z-index: 10;
  }
  
  .detail-area::before {
    top: 12px;
  }
  
  .detail-area::after {
    bottom: 12px;
  }
  
  /* Physical unit texture overlay */
  .detail-area:nth-child(1)::after {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: 
      repeating-linear-gradient(
        45deg,
        transparent,
        transparent 1px,
        rgba(255, 255, 255, 0.01) 1px,
        rgba(255, 255, 255, 0.01) 2px
      );
    pointer-events: none;
    z-index: 1;
  }
  
  .unit-label {
    text-align: center;
    background: 
      linear-gradient(to bottom, #4a4a4a, #2a2a2a),
      #3a3a3a;
    padding: clamp(8px, 1.5vh, 12px);
    border-radius: 4px;
    border: 1px solid #555;
    box-shadow: 
      inset 0 1px 0 rgba(255, 255, 255, 0.1),
      0 2px 4px rgba(0, 0, 0, 0.3);
  }
  
  
  /* Status LEDs */
  .status-leds {
    display: flex;
    justify-content: space-around;
    background: #1a1a1a;
    padding: clamp(8px, 1.5vh, 12px);
    border-radius: 4px;
    border: 1px solid #333;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.5);
  }
  
  .led-group {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }
  
  .led {
    width: clamp(8px, 1.5vw, 12px);
    height: clamp(8px, 1.5vw, 12px);
    border-radius: 50%;
    background: #333;
    border: 1px solid #555;
    box-shadow: 
      inset 0 1px 2px rgba(0, 0, 0, 0.5),
      0 0 0 1px rgba(255, 255, 255, 0.1);
    transition: all 0.3s;
  }
  
  .led.active {
    background: radial-gradient(circle, #0f0 30%, #0a0 70%);
    box-shadow: 
      inset 0 1px 2px rgba(0, 0, 0, 0.3),
      0 0 8px #0f0,
      0 0 16px rgba(0, 255, 0, 0.5);
  }
  
  .led-label {
    font-size: clamp(7px, 1vw, 9px);
    color: #888;
    letter-spacing: 0.1em;
    font-weight: bold;
  }
  
  /* CRT Display */
  .crt-display {
    flex: 1;
    background: #0a0a0a;
    padding: clamp(12px, 2vw, 20px);
    border-radius: 6px;
    border: 2px solid #333;
    box-shadow: 
      inset 0 4px 8px rgba(0, 0, 0, 0.8),
      0 2px 4px rgba(0, 0, 0, 0.3);
    position: relative;
  }
  
  /* Add screen label */
  .crt-display::before {
    content: 'DATA TERMINAL';
    position: absolute;
    top: -8px;
    left: 12px;
    background: #1a1a1a;
    padding: 2px 8px;
    font-size: 6px;
    color: #0a0;
    letter-spacing: 0.5px;
    font-weight: bold;
    border: 1px solid #333;
    border-radius: 2px;
    text-shadow: 0 0 3px #0a0;
  }
  
  .crt-bezel {
    background: 
      radial-gradient(ellipse at center, #1a1a1a 0%, #0a0a0a 70%);
    padding: clamp(8px, 1.5vw, 15px);
    border-radius: 4px;
    position: relative;
  }
  
  .crt-screen {
    background: #000;
    border-radius: 3px;
    position: relative;
    overflow: hidden;
    min-height: clamp(120px, 20vh, 200px);
    border: 1px solid #222;
    box-shadow: 
      inset 0 0 20px rgba(0, 255, 0, 0.1),
      inset 0 0 40px rgba(0, 255, 0, 0.05);
  }
  
  /* CRT scan lines effect */
  .scan-lines {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: repeating-linear-gradient(
      0deg,
      transparent,
      transparent 2px,
      rgba(0, 255, 0, 0.03) 2px,
      rgba(0, 255, 0, 0.03) 4px
    );
    pointer-events: none;
    z-index: 2;
  }
  
  .crt-content {
    padding: clamp(8px, 1.5vw, 15px);
    font-family: 'Share Tech Mono', monospace;
    font-size: clamp(8px, 1.2vw, 11px);
    line-height: 1.4;
    position: relative;
    z-index: 1;
    min-height: 100%;
    display: flex;
    flex-direction: column;
  }
  
  .crt-header {
    color: #0f0;
    margin-bottom: clamp(8px, 1.5vh, 12px);
    text-shadow: 0 0 5px #0f0;
  }
  
  .terminal-prompt {
    color: #0a0;
  }
  
  .cursor {
    animation: blink 1s infinite;
  }
  
  @keyframes blink {
    0%, 50% { opacity: 1; }
    51%, 100% { opacity: 0; }
  }
  
  .data-readout {
    flex: 1;
  }
  
  .data-line {
    display: flex;
    margin-bottom: clamp(3px, 0.5vh, 6px);
    align-items: flex-start;
  }
  
  .data-label {
    color: #0a0;
    min-width: clamp(30px, 8vw, 60px);
    margin-right: clamp(5px, 1vw, 10px);
    text-shadow: 0 0 3px #0a0;
  }
  
  .data-value {
    color: #0f0;
    text-shadow: 0 0 2px #0f0;
    word-break: break-all;
    flex: 1;
  }
  
  .data-value.reaction-display {
    font-size: clamp(14px, 2.5vw, 20px);
    text-align: center;
  }
  
  .data-block {
    margin-bottom: clamp(8px, 1.5vh, 12px);
  }
  
  .data-text {
    color: #0f0;
    margin-top: clamp(3px, 0.5vh, 6px);
    word-break: break-word;
    text-shadow: 0 0 1px #0f0;
  }
  
  /* CRT Standby */
  .crt-standby {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  
  .standby-text {
    text-align: center;
    color: #0a0;
    opacity: 0.7;
  }
  
  .standby-sub {
    font-size: clamp(7px, 1vw, 9px);
    margin-top: clamp(5px, 1vh, 8px);
    opacity: 0.5;
  }
  
  .scan-wave {
    width: clamp(40px, 8vw, 60px);
    height: 2px;
    background: linear-gradient(90deg, transparent, #0f0, transparent);
    margin: 0 auto clamp(8px, 1.5vh, 12px);
    animation: scan 2s infinite;
  }
  
  @keyframes scan {
    0% { opacity: 0.3; transform: scaleX(0.5); }
    50% { opacity: 1; transform: scaleX(1); }
    100% { opacity: 0.3; transform: scaleX(0.5); }
  }
  
  /* Physical Controls */
  .crt-controls {
    display: flex;
    justify-content: space-around;
    margin-top: clamp(8px, 1.5vh, 12px);
    padding: clamp(5px, 1vh, 8px);
    background: #1a1a1a;
    border-radius: 3px;
    border: 1px solid #333;
  }
  
  .knob-group {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: clamp(3px, 0.5vh, 5px);
  }
  
  .knob {
    width: clamp(20px, 3vw, 30px);
    height: clamp(20px, 3vw, 30px);
    background: 
      radial-gradient(circle at 30% 30%, #6a6a6a, #2a2a2a);
    border-radius: 50%;
    border: 1px solid #555;
    position: relative;
    cursor: pointer;
    box-shadow: 
      inset 0 1px 0 rgba(255, 255, 255, 0.2),
      0 2px 4px rgba(0, 0, 0, 0.3);
  }
  
  .knob::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 50%;
    transform: translateX(-50%);
    width: 2px;
    height: clamp(6px, 1vw, 8px);
    background: #ddd;
    border-radius: 1px;
  }
  
  .knob-label {
    font-size: clamp(6px, 0.8vw, 8px);
    color: #888;
    letter-spacing: 0.05em;
    font-weight: bold;
  }
  
  /* Hardware Switches */
  .hardware-switches {
    display: flex;
    justify-content: space-around;
    background: #1a1a1a;
    padding: clamp(8px, 1.5vh, 12px);
    border-radius: 4px;
    border: 1px solid #333;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.5);
  }
  
  .switch-group {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: clamp(3px, 0.5vh, 5px);
  }
  
  .toggle-switch {
    width: clamp(30px, 4vw, 40px);
    height: clamp(15px, 2vh, 20px);
    background: #0a0a0a;
    border: 1px solid #333;
    border-radius: clamp(8px, 1vh, 10px);
    position: relative;
    cursor: pointer;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.8);
  }
  
  .switch-lever {
    position: absolute;
    top: 1px;
    left: 2px;
    width: clamp(12px, 1.5vw, 16px);
    height: clamp(11px, 1.5vh, 16px);
    background: 
      linear-gradient(to bottom, #5a5a5a, #3a3a3a);
    border-radius: 50%;
    border: 1px solid #666;
    transition: all 0.2s;
    box-shadow: 
      0 1px 2px rgba(0, 0, 0, 0.5),
      inset 0 1px 0 rgba(255, 255, 255, 0.2);
  }
  
  .toggle-switch.active .switch-lever {
    left: calc(100% - clamp(14px, 2vw, 18px));
    background: 
      linear-gradient(to bottom, #6a6a6a, #4a4a4a);
  }
  
  .switch-label {
    font-size: clamp(6px, 0.8vw, 8px);
    color: #888;
    letter-spacing: 0.05em;
    font-weight: bold;
  }
  
  @media (max-width: 768px) {
    .detail-area {
      border-left: none;
      border-top: 2px solid #333;
      max-height: 40vh;
    }
  }
  
  @media (orientation: portrait) and (max-height: 700px) {
    .detail-area {
      max-height: 35vh;
    }
  }
  
  /* Tape Deck Conveyor */
  .conveyor-area {
    grid-area: conveyor;
    background: 
      linear-gradient(180deg, #2a2a2a 0%, #1f1f1f 50%, #1a1a1a 100%);
    border: 2px solid #444;
    border-radius: 4px;
    padding: clamp(10px, 2vw, 15px);
    position: relative;
    overflow: hidden;
    box-shadow: 
      inset 0 2px 0 rgba(255, 255, 255, 0.1),
      inset 0 -2px 0 rgba(0, 0, 0, 0.3),
      0 2px 4px rgba(0, 0, 0, 0.2);
  }
  
  /* Conveyor mounting holes */
  .conveyor-area::before,
  .conveyor-area::after {
    content: '';
    position: absolute;
    top: 8px;
    width: 6px;
    height: 6px;
    background: #0a0a0a;
    border-radius: 50%;
    border: 1px solid #333;
    box-shadow: 
      inset 0 1px 2px rgba(0, 0, 0, 0.8),
      0 1px 0 rgba(255, 255, 255, 0.1);
    z-index: 10;
  }
  
  .conveyor-area::before {
    left: 8px;
  }
  
  .conveyor-area::after {
    right: 8px;
  }
  
  .conveyor-header {
    display: flex;
    align-items: baseline;
    gap: 4px;
    padding: 2px 6px;
    margin-bottom: 4px;
  }
  
  .conveyor-title {
    font-size: 8px;
    color: #888;
    font-weight: bold;
    letter-spacing: 0.1em;
  }
  
  .conveyor-version {
    font-size: 6px;
    color: #666;
  }
  
  .conveyor-mechanism {
    display: flex;
    flex-direction: column;
    gap: clamp(8px, 1.5vh, 12px);
  }
  
  .conveyor-track {
    background: #0a0a0a;
    border: 2px solid #333;
    border-radius: 6px;
    padding: clamp(8px, 1.5vw, 12px);
    position: relative;
    box-shadow: inset 0 4px 8px rgba(0, 0, 0, 0.8);
  }
  
  /* Motor housing */
  .motor-housing {
    position: absolute;
    left: clamp(8px, 2vw, 15px);
    top: 50%;
    transform: translateY(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
  }
  
  .motor {
    width: clamp(16px, 2.5vw, 24px);
    height: clamp(16px, 2.5vw, 24px);
    background: 
      radial-gradient(circle at 30% 30%, #4a4a4a, #222);
    border: 2px solid #555;
    border-radius: 50%;
    position: relative;
    box-shadow: 
      0 2px 4px rgba(0, 0, 0, 0.4),
      inset 0 1px 0 rgba(255, 255, 255, 0.2);
  }
  
  .motor::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 50%;
    transform: translateX(-50%);
    width: 2px;
    height: clamp(4px, 1vw, 6px);
    background: #ddd;
    border-radius: 1px;
  }
  
  .motor.running {
    animation: motor-spin 0.2s linear infinite;
  }
  
  @keyframes motor-spin {
    from { transform: translateY(-50%) rotate(0deg); }
    to { transform: translateY(-50%) rotate(360deg); }
  }
  
  .motor-label {
    font-size: clamp(5px, 0.8vw, 7px);
    color: #888;
    font-weight: bold;
    letter-spacing: 0.5px;
  }
  
  /* Conveyor belt */
  .conveyor-belt {
    margin-left: clamp(40px, 6vw, 60px);
    position: relative;
    background: #111;
    border: 1px solid #222;
    border-radius: 4px;
    overflow: hidden;
  }
  
  .belt-surface {
    display: flex;
    gap: 2px;
    padding: clamp(4px, 0.8vw, 6px);
    position: relative;
    z-index: 2;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: thin;
    scrollbar-color: #333 #111;
  }
  
  .belt-surface::-webkit-scrollbar {
    height: 6px;
  }
  
  .belt-surface::-webkit-scrollbar-track {
    background: #111;
    border-radius: 3px;
  }
  
  .belt-surface::-webkit-scrollbar-thumb {
    background: #333;
    border-radius: 3px;
  }
  
  .belt-surface::-webkit-scrollbar-thumb:hover {
    background: #444;
  }
  
  .belt-texture {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: repeating-linear-gradient(
      90deg,
      #111 0px,
      #111 1px,
      #0a0a0a 1px,
      #0a0a0a 3px
    );
    z-index: 1;
  }
  
  /* Tape slots */
  .tape-slot {
    position: relative;
    background: #0a0a0a;
    border: 1px solid #333;
    border-radius: 3px;
    padding: clamp(2px, 0.5vw, 4px);
    min-height: clamp(60px, 12vh, 80px);
    width: clamp(80px, 15vw, 120px);
    min-width: 80px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.8);
    transition: all 0.2s;
  }
  
  .tape-slot.active {
    border-color: #0a0;
    box-shadow: 
      inset 0 2px 4px rgba(0, 0, 0, 0.8),
      0 0 8px rgba(0, 255, 0, 0.3);
  }
  
  .slot-number {
    position: absolute;
    top: 2px;
    left: 2px;
    font-size: clamp(6px, 1vw, 8px);
    color: #666;
    font-weight: bold;
  }
  
  /* Cassette body */
  .cassette-body {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    position: relative;
  }
  
  .cassette-face {
    flex: 1;
    background: 
      linear-gradient(135deg, #2a2a2a 0%, #1a1a1a 100%);
    border: 1px solid #444;
    border-radius: 2px;
    padding: clamp(2px, 0.4vw, 3px);
    display: flex;
    flex-direction: column;
    gap: 2px;
    position: relative;
    box-shadow: 
      0 1px 3px rgba(0, 0, 0, 0.5),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
  }
  
  .cassette-window {
    background: #000;
    height: clamp(8px, 1.5vh, 12px);
    border: 1px solid #555;
    border-radius: 1px;
  }
  
  .cassette-reels {
    display: flex;
    justify-content: space-between;
    padding: 0 clamp(2px, 0.4vw, 3px);
    flex: 1;
    align-items: center;
  }
  
  .reel {
    width: clamp(8px, 1.2vw, 12px);
    height: clamp(8px, 1.2vw, 12px);
    background: #333;
    border: 1px solid #555;
    border-radius: 50%;
    position: relative;
  }
  
  .reel::after {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: clamp(3px, 0.5vw, 4px);
    height: clamp(3px, 0.5vw, 4px);
    background: #111;
    border-radius: 50%;
  }
  
  .reel.spinning {
    animation: reel-spin 0.5s linear infinite;
  }
  
  @keyframes reel-spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
  
  .cassette-label-strip {
    background: #f0f0f0;
    height: clamp(8px, 1.5vh, 12px);
    border-radius: 1px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-top: 2px;
  }
  
  .cassette-name {
    font-size: clamp(4px, 0.7vw, 6px);
    color: #000;
    font-weight: bold;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  
  /* Eject lever */
  .eject-lever {
    position: absolute;
    top: 2px;
    right: 2px;
    width: clamp(8px, 1.2vw, 12px);
    height: clamp(12px, 2vh, 16px);
    background: 
      linear-gradient(to bottom, #666, #333);
    border: 1px solid #555;
    border-radius: 2px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: all 0.2s;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
  }
  
  .tape-slot:hover .eject-lever {
    opacity: 0.8;
  }
  
  .eject-lever:hover {
    opacity: 1 !important;
    transform: translateY(-1px);
  }
  
  .lever-handle {
    width: 2px;
    height: clamp(6px, 1vh, 8px);
    background: #aaa;
    border-radius: 1px;
  }
  
  /* Empty slot mechanism */
  .empty-slot-mechanism {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: clamp(2px, 0.4vh, 3px);
  }
  
  .slot-guides {
    display: flex;
    justify-content: space-between;
    width: 80%;
    height: clamp(4px, 0.8vh, 6px);
  }
  
  .guide {
    width: clamp(6px, 1vw, 8px);
    height: 100%;
    background: #333;
    border: 1px solid #444;
    border-radius: 1px;
  }
  
  .empty-indicator {
    font-size: clamp(5px, 0.8vw, 7px);
    color: #444;
    font-weight: bold;
    letter-spacing: 0.5px;
  }
  
  /* Selection LED */
  .selection-led {
    position: absolute;
    bottom: 2px;
    right: 2px;
    width: clamp(4px, 0.6vw, 6px);
    height: clamp(4px, 0.6vw, 6px);
    background: #222;
    border: 1px solid #333;
    border-radius: 50%;
    transition: all 0.3s;
  }
  
  .selection-led.selected {
    background: radial-gradient(circle, #f80 40%, #f40 100%);
    box-shadow: 
      0 0 6px #f80,
      0 0 12px rgba(255, 136, 0, 0.5);
  }
  
  /* Slot selector (invisible clickable area) */
  .slot-selector {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: transparent;
    border: none;
    cursor: pointer;
    z-index: 5;
  }
  
  .slot-selector:disabled {
    cursor: default;
  }
  
  /* Conveyor controls */
  .conveyor-controls {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: clamp(6px, 1vh, 8px);
  }
  
  .control-group {
    display: flex;
    align-items: center;
    gap: clamp(6px, 1vw, 10px);
  }
  
  .conveyor-btn {
    background: 
      linear-gradient(145deg, #4a4a4a 0%, #3a3a3a 45%, #2a2a2a 100%);
    border: 2px solid #666;
    border-radius: 4px;
    padding: clamp(4px, 0.8vw, 6px) clamp(8px, 1.5vw, 12px);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: clamp(2px, 0.4vw, 3px);
    transition: all 0.15s;
    box-shadow: 
      0 2px 4px rgba(0, 0, 0, 0.4),
      inset 0 1px 0 rgba(255, 255, 255, 0.2);
  }
  
  .conveyor-btn:hover {
    background: 
      linear-gradient(145deg, #5a5a5a 0%, #4a4a4a 45%, #3a3a3a 100%);
    transform: translateY(-1px);
  }
  
  .conveyor-btn:active {
    transform: translateY(1px);
    box-shadow: 
      0 1px 2px rgba(0, 0, 0, 0.4),
      inset 0 1px 3px rgba(0, 0, 0, 0.4);
  }
  
  .btn-symbol {
    font-size: clamp(10px, 1.5vw, 14px);
    color: #0f0;
    font-weight: bold;
  }
  
  .btn-text {
    font-size: clamp(6px, 1vw, 8px);
    color: #ccc;
    font-weight: bold;
    letter-spacing: 0.5px;
  }
  
  .speed-indicator {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }
  
  .speed-led {
    width: clamp(6px, 1vw, 8px);
    height: clamp(6px, 1vw, 8px);
    background: #222;
    border: 1px solid #333;
    border-radius: 50%;
    transition: all 0.3s;
  }
  
  .speed-led.active {
    background: radial-gradient(circle, #0f0 40%, #0a0 100%);
    box-shadow: 
      0 0 6px #0f0,
      0 0 12px rgba(0, 255, 0, 0.5);
  }
  
  .speed-label {
    font-size: clamp(5px, 0.8vw, 7px);
    color: #888;
    font-weight: bold;
    letter-spacing: 0.5px;
  }

  .hidden {
    display: none;
  }
  
  /* Ultra-wide screens */
  @media (min-width: 1800px) {
    .console-grid {
      grid-template-columns: minmax(250px, 18vw) 1fr minmax(350px, 22vw);
    }
  }
  
  /* Large tablet landscape */
  @media (max-width: 1024px) and (orientation: landscape) {
    .top-panel {
      height: clamp(35px, 6vh, 50px);
      padding: 0 clamp(10px, 2vw, 20px);
    }
  }
  
  /* Tablet portrait and small laptops */
  @media (max-width: 1024px) and (orientation: portrait) {
    .console-grid {
      grid-template-columns: 1fr;
      grid-template-rows: 1fr minmax(200px, 35vh) auto;
      grid-template-areas: 
        "center"
        "detail"
        "sidebar";
    }
  }
  
  /* Phone landscape */
  @media (max-width: 768px) and (orientation: landscape) {
    .console-grid {
      grid-template-columns: minmax(150px, 30vw) 1fr;
      grid-template-rows: 1fr auto;
      grid-template-areas: 
        "sidebar center"
        "detail detail";
    }
    
    .detail-area {
      max-height: 25vh;
      border-left: none;
      border-top: 2px solid #333;
    }
  }
  
  /* Very small screens */
  @media (max-width: 360px) {
    .top-panel {
      height: clamp(35px, 10vh, 45px);
      padding: 0 clamp(8px, 2vw, 15px);
    }
    
    .brand-name {
      font-size: clamp(12px, 4vw, 18px);
    }
    
    .preset-buttons {
      grid-template-columns: 1fr 1fr;
    }
    
    .bank-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>