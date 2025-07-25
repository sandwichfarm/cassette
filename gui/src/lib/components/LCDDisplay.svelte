<script>
  import { cassettes, selectedCassettes, currentCassette, terminalOutput } from '../stores/cassette.js';
  import { onMount, onDestroy } from 'svelte';
  
  let displayMode = 'status';
  let displayText = '';
  let scrollOffset = 0;
  let scrollInterval;
  
  $: cassetteCount = $cassettes.size;
  $: selectedCount = $selectedCassettes.size;
  $: currentName = $currentCassette?.name || 'NO CASSETTE';
  
  // Get latest terminal output
  $: latestOutput = $terminalOutput.length > 0 ? 
    $terminalOutput[$terminalOutput.length - 1] : null;
  
  function updateDisplay() {
    switch (displayMode) {
      case 'status':
        displayText = `CASSETTES: ${cassetteCount} | SELECTED: ${selectedCount}`;
        break;
      case 'current':
        displayText = `PLAYING: ${currentName}`;
        break;
      case 'output':
        if (latestOutput) {
          displayText = latestOutput.text.substring(0, 100);
        } else {
          displayText = 'NO OUTPUT';
        }
        break;
    }
  }
  
  $: updateDisplay(), $cassettes, $selectedCassettes, $currentCassette, $terminalOutput;
  
  function cycleMode() {
    const modes = ['status', 'current', 'output'];
    const currentIndex = modes.indexOf(displayMode);
    displayMode = modes[(currentIndex + 1) % modes.length];
  }
  
  // Scroll long text
  onMount(() => {
    scrollInterval = setInterval(() => {
      if (displayText.length > 40) {
        scrollOffset = (scrollOffset + 1) % (displayText.length - 20);
      }
    }, 200);
  });
  
  onDestroy(() => {
    clearInterval(scrollInterval);
  });
  
  $: scrolledText = displayText.length > 40 ? 
    displayText.substring(scrollOffset, scrollOffset + 40) : displayText;
</script>

<div class="lcd-display" on:click={cycleMode}>
  <div class="lcd-screen">
    <div class="lcd-content">
      <div class="lcd-text">{scrolledText}</div>
      <div class="lcd-mode">{displayMode.toUpperCase()}</div>
    </div>
    
    <!-- LCD grid effect -->
    <div class="lcd-grid"></div>
  </div>
  
  <div class="display-label">SYSTEM MONITOR</div>
</div>

<style>
  .lcd-display {
    background: #1a1a1a;
    padding: 15px;
    border-radius: 4px;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.5);
    cursor: pointer;
    user-select: none;
  }
  
  .lcd-screen {
    background: #001a00;
    border: 2px solid #333;
    border-radius: 2px;
    padding: 20px;
    position: relative;
    overflow: hidden;
    box-shadow: 
      inset 0 2px 8px rgba(0, 0, 0, 0.8),
      0 0 20px rgba(0, 255, 0, 0.1);
  }
  
  .lcd-content {
    position: relative;
    z-index: 2;
  }
  
  .lcd-text {
    font-family: 'Share Tech Mono', monospace;
    font-size: 16px;
    color: #0f0;
    text-shadow: 
      0 0 3px #0f0,
      0 0 5px #0f0;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    white-space: nowrap;
    overflow: hidden;
    margin-bottom: 8px;
  }
  
  .lcd-mode {
    font-family: 'Share Tech Mono', monospace;
    font-size: 10px;
    color: #0a0;
    text-align: right;
    opacity: 0.7;
  }
  
  /* LCD pixel grid effect */
  .lcd-grid {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-image: 
      repeating-linear-gradient(
        0deg,
        transparent,
        transparent 2px,
        rgba(0, 0, 0, 0.1) 2px,
        rgba(0, 0, 0, 0.1) 3px
      ),
      repeating-linear-gradient(
        90deg,
        transparent,
        transparent 2px,
        rgba(0, 0, 0, 0.1) 2px,
        rgba(0, 0, 0, 0.1) 3px
      );
    pointer-events: none;
  }
  
  .display-label {
    font-size: 10px;
    color: #666;
    text-align: center;
    margin-top: 8px;
    letter-spacing: 0.1em;
  }
  
  /* Hover effect */
  .lcd-display:hover .lcd-screen {
    box-shadow: 
      inset 0 2px 8px rgba(0, 0, 0, 0.8),
      0 0 30px rgba(0, 255, 0, 0.2);
  }
  
  .lcd-display:active .lcd-screen {
    transform: scale(0.98);
  }
</style>