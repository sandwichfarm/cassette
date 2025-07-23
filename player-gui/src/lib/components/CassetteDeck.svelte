<script>
  import { currentCassette, isPlaying } from '../stores/cassette.js';
  import TapeReel from './TapeReel.svelte';
  import VUMeter from './VUMeter.svelte';
  
  let spinning = false;
  
  $: spinning = $isPlaying;
  $: cassette = $currentCassette;
</script>

<div class="cassette-deck">
  <!-- Main deck body -->
  <div class="deck-body">
    <!-- VU Meters -->
    <div class="vu-section">
      <VUMeter active={spinning} channel="L" />
      <VUMeter active={spinning} channel="R" />
    </div>
    
    <!-- Tape mechanism -->
    <div class="tape-section">
      <TapeReel spinning={spinning} side="left" />
      
      <div class="cassette-slot">
        {#if cassette}
          <div class="cassette-loaded">
            <div class="cassette-label">
              <div class="label-stripe"></div>
              <div class="label-content">
                <div class="cassette-name">{cassette.name}</div>
                <div class="cassette-info">
                  <span>v{cassette.version}</span>
                  <span>•</span>
                  <span>{cassette.eventCount} events</span>
                </div>
                <div class="cassette-desc">{cassette.description}</div>
              </div>
              <div class="label-stripe"></div>
            </div>
          </div>
        {:else}
          <div class="empty-slot">
            <div class="slot-text">NO CASSETTE</div>
          </div>
        {/if}
      </div>
      
      <TapeReel spinning={spinning} side="right" />
    </div>
    
    <!-- Transport controls -->
    <div class="transport-controls">
      <button 
        class="transport-btn record"
        disabled={!cassette}
        on:click={() => isPlaying.set(true)}
      >
        <span class="btn-icon">●</span>
        <span class="btn-label">REC</span>
      </button>
      
      <button 
        class="transport-btn play"
        disabled={!cassette || $isPlaying}
        on:click={() => isPlaying.set(true)}
      >
        <span class="btn-icon">▶</span>
        <span class="btn-label">PLAY</span>
      </button>
      
      <button 
        class="transport-btn stop"
        disabled={!$isPlaying}
        on:click={() => isPlaying.set(false)}
      >
        <span class="btn-icon">■</span>
        <span class="btn-label">STOP</span>
      </button>
      
      <button 
        class="transport-btn pause"
        disabled={!$isPlaying}
        on:click={() => isPlaying.set(false)}
      >
        <span class="btn-icon">⏸</span>
        <span class="btn-label">PAUSE</span>
      </button>
    </div>
    
    <!-- Status display -->
    <div class="status-display">
      <div class="status-item">
        <span class="status-label">STATUS:</span>
        <span class="status-value {$isPlaying ? 'active' : ''}">
          {$isPlaying ? 'PLAYING' : cassette ? 'READY' : 'EMPTY'}
        </span>
      </div>
      <div class="status-item">
        <span class="status-label">MODE:</span>
        <span class="status-value">NIP-01</span>
      </div>
    </div>
  </div>
</div>

<style>
  .cassette-deck {
    width: 100%;
    max-width: 800px;
    margin: 0 auto;
  }
  
  .deck-body {
    background: linear-gradient(135deg, #1a1a1a 0%, #0a0a0a 100%);
    border: 3px solid #333;
    border-radius: 8px;
    padding: 30px;
    box-shadow: 
      0 0 50px rgba(0, 255, 0, 0.2),
      inset 0 0 30px rgba(0, 0, 0, 0.8);
  }
  
  /* VU Meters */
  .vu-section {
    display: flex;
    justify-content: center;
    gap: 20px;
    margin-bottom: 30px;
  }
  
  /* Tape section */
  .tape-section {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 20px;
    margin-bottom: 30px;
    padding: 20px;
    background: rgba(0, 0, 0, 0.5);
    border-radius: 8px;
  }
  
  .cassette-slot {
    width: 300px;
    height: 180px;
    background: #222;
    border: 2px solid #444;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    overflow: hidden;
  }
  
  .empty-slot {
    text-align: center;
    color: #666;
    font-size: 18px;
    letter-spacing: 0.2em;
  }
  
  .cassette-loaded {
    width: 100%;
    height: 100%;
    background: linear-gradient(135deg, #2a2a2a 0%, #1a1a1a 100%);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  
  .cassette-label {
    width: 90%;
    height: 80%;
    background: #f0f0f0;
    border-radius: 2px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
  }
  
  .label-stripe {
    height: 20px;
    background: repeating-linear-gradient(
      45deg,
      #ff0000,
      #ff0000 10px,
      #0000ff 10px,
      #0000ff 20px
    );
  }
  
  .label-content {
    flex: 1;
    padding: 15px;
    color: #000;
    display: flex;
    flex-direction: column;
    justify-content: center;
    font-family: 'Share Tech Mono', monospace;
  }
  
  .cassette-name {
    font-size: 20px;
    font-weight: bold;
    margin-bottom: 8px;
    text-transform: uppercase;
  }
  
  .cassette-info {
    font-size: 12px;
    color: #666;
    margin-bottom: 8px;
  }
  
  .cassette-desc {
    font-size: 11px;
    color: #888;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  
  /* Transport controls */
  .transport-controls {
    display: flex;
    justify-content: center;
    gap: 10px;
    margin-bottom: 20px;
  }
  
  .transport-btn {
    background: linear-gradient(to bottom, #3a3a3a, #1a1a1a);
    border: 3px solid #555;
    border-radius: 8px;
    padding: 15px 25px;
    color: #ccc;
    cursor: pointer;
    transition: all 0.1s;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    min-width: 80px;
    font-family: 'Share Tech Mono', monospace;
  }
  
  .transport-btn:hover:not(:disabled) {
    background: linear-gradient(to bottom, #4a4a4a, #2a2a2a);
    border-color: #666;
  }
  
  .transport-btn:active:not(:disabled) {
    transform: translateY(2px);
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.5);
  }
  
  .transport-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
  
  .transport-btn.record {
    color: #f00;
  }
  
  .transport-btn.play {
    color: #0f0;
  }
  
  .btn-icon {
    font-size: 24px;
  }
  
  .btn-label {
    font-size: 10px;
    letter-spacing: 0.1em;
  }
  
  /* Status display */
  .status-display {
    display: flex;
    justify-content: center;
    gap: 40px;
    padding: 15px;
    background: #000;
    border-radius: 4px;
    border: 1px solid #333;
  }
  
  .status-item {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  
  .status-label {
    color: #666;
    font-size: 12px;
    letter-spacing: 0.1em;
  }
  
  .status-value {
    color: #0a0;
    font-size: 14px;
    font-weight: bold;
    letter-spacing: 0.1em;
  }
  
  .status-value.active {
    color: #0f0;
    text-shadow: 0 0 5px #0f0;
    animation: pulse 1s infinite;
  }
  
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.7; }
  }
  
  /* Responsive */
  @media (max-width: 768px) {
    .deck-body {
      padding: 20px;
    }
    
    .cassette-slot {
      width: 200px;
      height: 120px;
    }
    
    .transport-btn {
      padding: 10px 15px;
      min-width: 60px;
    }
    
    .btn-icon {
      font-size: 20px;
    }
  }
</style>