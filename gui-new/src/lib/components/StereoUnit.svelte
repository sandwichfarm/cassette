<script>
  import { cassettes, currentCassette, selectedCassettes, sendRequest, terminalOutput, loadCassetteFile, ejectCassette, isPlaying } from '../stores/cassette.js';
  import { soundEnabled, soundEffects } from '../stores/sound.js';
  import LCDDisplay from './LCDDisplay.svelte';
  import BankButton from './BankButton.svelte';
  import TapeTransport from './TapeTransport.svelte';
  import { onMount, onDestroy } from 'svelte';
  
  let fileInput;
  let currentBank = null;
  let commandInput = '';
  let leftMeterHeight = 30;
  let rightMeterHeight = 30;
  let animationFrame;
  
  // Map cassettes to banks 1-9
  $: cassetteArray = Array.from($cassettes.values());
  $: bankCassettes = cassetteArray.slice(0, 9);
  
  // Animate VU meters when playing
  $: if ($isPlaying) {
    startMeterAnimation();
  } else {
    stopMeterAnimation();
  }
  
  function startMeterAnimation() {
    if (animationFrame) return; // Already running
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
    
    // Simulate audio levels with random fluctuations
    const baseLevel = 50;
    const variance = 30;
    
    leftMeterHeight = baseLevel + (Math.random() - 0.5) * variance;
    rightMeterHeight = baseLevel + (Math.random() - 0.5) * variance;
    
    // Add occasional peaks
    if (Math.random() < 0.1) {
      leftMeterHeight = 70 + Math.random() * 25;
    }
    if (Math.random() < 0.1) {
      rightMeterHeight = 70 + Math.random() * 25;
    }
    
    // Clamp values
    leftMeterHeight = Math.max(5, Math.min(95, leftMeterHeight));
    rightMeterHeight = Math.max(5, Math.min(95, rightMeterHeight));
    
    animationFrame = requestAnimationFrame(animateMeterFrame);
  }
  
  onDestroy(() => {
    stopMeterAnimation();
  });
  
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
  
  function selectBank(num) {
    const cassette = bankCassettes[num - 1];
    if (cassette) {
      currentBank = num;
      currentCassette.set(cassette);
      soundEffects.playButton();
      
      // Toggle selection
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
  
  async function handleCommand(e) {
    if (e.key === 'Enter' && commandInput.trim()) {
      await sendRequest(commandInput);
      commandInput = '';
    }
  }
</script>

<div class="stereo-unit">
  <!-- Top panel with brushed metal texture -->
  <div class="top-panel">
    <div class="brand">
      <div class="brand-name">CASSETTE</div>
      <div class="model">NEURAL DECK MK2</div>
    </div>
    
    <div class="controls">
      <label class="sound-toggle">
        <input type="checkbox" bind:checked={$soundEnabled} />
        <span class="toggle-label">SOUND</span>
      </label>
      <div class="power-indicator"></div>
    </div>
  </div>
  
  <!-- Main control surface -->
  <div class="control-surface">
    <!-- Left section - Bank selector -->
    <section class="bank-section">
      <!-- <div class="section-label">BANK SELECT</div> -->
      <div class="bank-grid">
        {#each [1, 2, 3, 4, 5, 6, 7, 8, 9] as num}
          <BankButton 
            {num}
            cassette={bankCassettes[num - 1]}
            active={currentBank === num}
            selected={bankCassettes[num - 1] && $selectedCassettes.has(bankCassettes[num - 1].id)}
            on:select={() => selectBank(num)}
            on:eject={() => ejectBank(num)}
          />
        {/each}
      </div>
      
      <div class="load-section">
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
    </section>
    
    <!-- Center section - Display and transport -->
    <section class="center-section">
      <div class="display-area">
        <LCDDisplay />
      </div>
      
      <div class="transport-area">
        <TapeTransport />
      </div>
      
      <!-- Command input -->
      <div class="command-section">
        <div class="command-label">COMMAND</div>
        <input 
          type="text"
          class="command-input"
          bind:value={commandInput}
          on:keydown={handleCommand}
          placeholder={'["REQ", "sub", {"kinds": [1]}]'}
        />
      </div>
    </section>
    
    <!-- Right section - VU meters -->
    <section class="meter-section">
      <div class="section-label">LEVELS</div>
      <div class="vu-meter left">
        <div class="meter-scale">
          <div class="meter-bar" style="height: {leftMeterHeight}%"></div>
        </div>
        <div class="meter-label">L</div>
      </div>
      <div class="vu-meter right">
        <div class="meter-scale">
          <div class="meter-bar" style="height: {rightMeterHeight}%"></div>
        </div>
        <div class="meter-label">R</div>
      </div>
    </section>
  </div>
  
  <!-- Bottom edge -->
  <div class="bottom-edge"></div>
</div>

<style>
  .stereo-unit {
    width: 100%;
    max-width: 1200px;
    background: #3a3a3a;
    border-radius: 8px;
    box-shadow: 
      0 10px 40px rgba(0, 0, 0, 0.5),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
  }
  
  /* Top panel - brushed metal */
  .top-panel {
    height: 60px;
    background: 
      repeating-linear-gradient(
        90deg,
        #4a4a4a,
        #4a4a4a 1px,
        #5a5a5a 1px,
        #5a5a5a 2px
      ),
      linear-gradient(
        180deg,
        rgba(255,255,255,0.05) 0%,
        transparent 50%,
        rgba(0,0,0,0.1) 100%
      ),
      #4a4a4a;
    border-radius: 8px 8px 0 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 30px;
    border-bottom: 2px solid #2a2a2a;
    position: relative;
    overflow: hidden;
  }
  
  /* Add subtle reflection */
  .top-panel::before {
    content: '';
    position: absolute;
    top: 0;
    left: -50%;
    right: -50%;
    height: 100%;
    background: linear-gradient(
      90deg,
      transparent 0%,
      rgba(255,255,255,0.03) 45%,
      rgba(255,255,255,0.05) 50%,
      rgba(255,255,255,0.03) 55%,
      transparent 100%
    );
    transform: skewX(-20deg);
    pointer-events: none;
  }
  
  .brand {
    color: #ddd;
  }
  
  .brand-name {
    font-size: 24px;
    font-weight: 300;
    letter-spacing: 0.2em;
  }
  
  .model {
    font-size: 10px;
    color: #999;
    letter-spacing: 0.1em;
  }
  
  .controls {
    display: flex;
    align-items: center;
    gap: 20px;
  }
  
  .sound-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }
  
  .sound-toggle input[type="checkbox"] {
    width: 40px;
    height: 20px;
    appearance: none;
    background: #222;
    border: 1px solid #444;
    border-radius: 10px;
    position: relative;
    cursor: pointer;
    transition: all 0.3s;
  }
  
  .sound-toggle input[type="checkbox"]::after {
    content: '';
    position: absolute;
    width: 16px;
    height: 16px;
    background: #666;
    border-radius: 50%;
    top: 1px;
    left: 2px;
    transition: all 0.3s;
  }
  
  .sound-toggle input[type="checkbox"]:checked {
    background: #0a0;
  }
  
  .sound-toggle input[type="checkbox"]:checked::after {
    left: 22px;
    background: #fff;
  }
  
  .toggle-label {
    font-size: 10px;
    color: #999;
    letter-spacing: 0.1em;
  }
  
  .power-indicator {
    width: 8px;
    height: 8px;
    background: #0f0;
    border-radius: 50%;
    box-shadow: 
      0 0 10px #0f0,
      inset 0 0 2px rgba(0, 0, 0, 0.3);
  }
  
  /* Main control surface */
  .control-surface {
    background: #2a2a2a;
    padding: 30px;
    display: grid;
    grid-template-columns: 240px 1fr 120px;
    gap: 30px;
  }
  
  /* Section styling */
  .section-label {
    font-size: 10px;
    color: #888;
    letter-spacing: 0.1em;
    margin-bottom: 15px;
    text-align: center;
  }
  
  /* Bank section */
  .bank-section {
    background: #1a1a1a;
    padding: 20px;
    border-radius: 4px;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.5);
  }
  
  .bank-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
    margin-bottom: 20px;
  }
  
  .load-section {
    margin-top: 20px;
  }
  
  .load-button {
    width: 100%;
    height: 40px;
    background: linear-gradient(to bottom, #4a4a4a, #3a3a3a);
    border: 1px solid #555;
    border-radius: 4px;
    color: #ccc;
    font-size: 12px;
    letter-spacing: 0.1em;
    cursor: pointer;
    transition: all 0.1s;
    box-shadow: 
      0 2px 4px rgba(0, 0, 0, 0.3),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
  }
  
  .load-button:hover {
    background: linear-gradient(to bottom, #5a5a5a, #4a4a4a);
  }
  
  .load-button:active {
    transform: translateY(1px);
    box-shadow: 
      0 1px 2px rgba(0, 0, 0, 0.3),
      inset 0 1px 2px rgba(0, 0, 0, 0.3);
  }
  
  /* Center section */
  .center-section {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  
  .display-area {
    flex: 1;
  }
  
  .transport-area {
    background: #1a1a1a;
    padding: 20px;
    border-radius: 4px;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.5);
  }
  
  /* Command section */
  .command-section {
    background: #1a1a1a;
    padding: 15px;
    border-radius: 4px;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.5);
  }
  
  .command-label {
    font-size: 10px;
    color: #888;
    letter-spacing: 0.1em;
    margin-bottom: 8px;
  }
  
  .command-input {
    width: 100%;
    background: #0a0a0a;
    border: 1px solid #333;
    border-radius: 2px;
    color: #0f0;
    padding: 8px 10px;
    font-family: 'Share Tech Mono', monospace;
    font-size: 12px;
    outline: none;
  }
  
  .command-input:focus {
    border-color: #555;
    box-shadow: 0 0 0 1px #555;
  }
  
  .command-input::placeholder {
    color: #0a0;
    opacity: 0.3;
  }
  
  /* VU Meters */
  .meter-section {
    background: #1a1a1a;
    padding: 20px;
    border-radius: 4px;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.5);
  }
  
  .vu-meter {
    margin-bottom: 20px;
  }
  
  .meter-scale {
    height: 100px;
    background: #0a0a0a;
    border: 1px solid #333;
    border-radius: 2px;
    position: relative;
    overflow: hidden;
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.8);
  }
  
  /* Scale markings */
  .meter-scale::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-image: repeating-linear-gradient(
      0deg,
      transparent,
      transparent 9px,
      rgba(255, 255, 255, 0.1) 9px,
      rgba(255, 255, 255, 0.1) 10px
    );
  }
  
  .meter-bar {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 30%;
    background: linear-gradient(
      to top,
      #0a0 0%,
      #0f0 70%,
      #ff0 85%,
      #f00 95%
    );
    transition: height 0.05s ease-out;
    box-shadow: 0 0 10px rgba(0, 255, 0, 0.5);
  }
  
  .meter-label {
    text-align: center;
    margin-top: 5px;
    font-size: 10px;
    color: #666;
  }
  
  /* Bottom edge */
  .bottom-edge {
    height: 20px;
    background: #1a1a1a;
    border-radius: 0 0 8px 8px;
    border-top: 1px solid #444;
  }
  
  .hidden {
    display: none;
  }
  
  /* Responsive */
  @media (max-width: 900px) {
    .control-surface {
      grid-template-columns: 1fr;
    }
    
    .bank-section {
      order: 2;
    }
    
    .center-section {
      order: 1;
    }
    
    .meter-section {
      display: none;
    }
  }
</style>