<script>
  import { onMount } from 'svelte';
  import { cassetteState, sendRequest, ejectCassette } from '../stores/cassette.js';
  import * as Tone from 'tone';
  
  let showRequestEditor = false;
  let requestText = JSON.stringify(["REQ", "sub1", { "kinds": [1], "limit": 10 }], null, 2);
  let synth;
  let clickSynth;
  let soundEnabled = false; // Disable sounds by default for better performance
  
  onMount(() => {
    // Lazy initialize Tone.js only when needed
    let initialized = false;
    
    const initializeTone = async () => {
      if (!initialized) {
        initialized = true;
        
        // Initialize Tone.js synthesizers for sound effects
        synth = new Tone.Synth({
          oscillator: { type: 'square' },
          envelope: { attack: 0.01, decay: 0.1, sustain: 0.2, release: 0.1 }
        }).toDestination();
        
        clickSynth = new Tone.Synth({
          oscillator: { type: 'sine' },
          envelope: { attack: 0.005, decay: 0.05, sustain: 0, release: 0.05 }
        }).toDestination();
      }
    };
    
    // Store the initializer for use in button handlers
    window._initializeTone = initializeTone;
    
    return () => {
      if (synth) synth.dispose();
      if (clickSynth) clickSynth.dispose();
      delete window._initializeTone;
    };
  });
  
  async function playButtonSound(note = 'C4') {
    if (!soundEnabled) return; // Skip if sounds are disabled
    
    // Initialize Tone.js if needed
    if (window._initializeTone) {
      await window._initializeTone();
    }
    
    if (!clickSynth) return; // Skip if not initialized
    
    if (Tone.context.state !== 'running') {
      await Tone.start();
    }
    clickSynth.triggerAttackRelease(note, '16n');
  }
  
  async function playLoadSound() {
    if (!soundEnabled) return; // Skip if sounds are disabled
    
    // Initialize Tone.js if needed
    if (window._initializeTone) {
      await window._initializeTone();
    }
    
    if (!synth) return; // Skip if not initialized
    
    if (Tone.context.state !== 'running') {
      await Tone.start();
    }
    // Play a sequence for cassette loading
    const now = Tone.now();
    synth.triggerAttackRelease('C4', '8n', now);
    synth.triggerAttackRelease('E4', '8n', now + 0.1);
    synth.triggerAttackRelease('G4', '8n', now + 0.2);
  }
  
  async function playEjectSound() {
    if (!soundEnabled) return; // Skip if sounds are disabled
    
    // Initialize Tone.js if needed
    if (window._initializeTone) {
      await window._initializeTone();
    }
    
    if (!synth) return; // Skip if not initialized
    
    if (Tone.context.state !== 'running') {
      await Tone.start();
    }
    // Play a descending sequence for eject
    const now = Tone.now();
    synth.triggerAttackRelease('G4', '8n', now);
    synth.triggerAttackRelease('E4', '8n', now + 0.1);
    synth.triggerAttackRelease('C4', '8n', now + 0.2);
  }
  
  async function handlePlay() {
    if ($cassetteState.loaded) {
      await playButtonSound('E4');
      cassetteState.update(s => ({ ...s, playing: true }));
      showRequestEditor = true;
      await playLoadSound();
    }
  }
  
  async function handleStop() {
    await playButtonSound('C4');
    cassetteState.update(s => ({ ...s, playing: false }));
    showRequestEditor = false;
  }
  
  async function handleRewind() {
    await playButtonSound('A3');
    cassetteState.update(s => ({ ...s, responses: [] }));
  }
  
  async function handleEject() {
    await playEjectSound();
    ejectCassette();
    showRequestEditor = false;
  }
  
  async function handleSendRequest() {
    try {
      await playButtonSound('G4');
      await sendRequest(requestText);
    } catch (error) {
      console.error('Failed to send request:', error);
    }
  }
</script>

<div class="control-panel">
  <div class="button-row">
    <button 
      class="control-button play"
      on:click={handlePlay}
      disabled={!$cassetteState.loaded || $cassetteState.playing}
    >
      <span class="icon">▶</span>
      <span class="label">PLAY</span>
    </button>
    
    <button 
      class="control-button stop"
      on:click={handleStop}
      disabled={!$cassetteState.playing}
    >
      <span class="icon">■</span>
      <span class="label">STOP</span>
    </button>
    
    <button 
      class="control-button rewind"
      on:click={handleRewind}
      disabled={!$cassetteState.loaded}
    >
      <span class="icon">◀◀</span>
      <span class="label">REWIND</span>
    </button>
    
    <button 
      class="control-button eject"
      on:click={handleEject}
      disabled={!$cassetteState.loaded}
    >
      <span class="icon">⏏</span>
      <span class="label">EJECT</span>
    </button>
  </div>
  
  {#if showRequestEditor}
    <div class="request-editor">
      <textarea 
        bind:value={requestText}
        class="request-input"
        placeholder="Enter NIP-01 request JSON..."
      />
      <button 
        class="send-button"
        on:click={handleSendRequest}
      >
        SEND REQUEST
      </button>
    </div>
  {/if}
</div>

<style>
  .control-panel {
    background: #222;
    border-radius: 8px;
    padding: 16px;
    box-shadow: inset 0 2px 4px rgba(0,0,0,0.5);
  }

  .button-row {
    display: flex;
    gap: 8px;
    justify-content: center;
  }

  .control-button {
    background: linear-gradient(to bottom, #4a4a4a, #2a2a2a);
    border: 2px solid #555;
    border-radius: 6px;
    padding: 12px 20px;
    color: #c0c0c0;
    font-weight: bold;
    cursor: pointer;
    transition: all 0.1s ease;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    min-width: 80px;
    box-shadow: 
      0 2px 4px rgba(0,0,0,0.3),
      inset 0 1px 1px rgba(255,255,255,0.1);
  }

  .control-button:hover:not(:disabled) {
    background: linear-gradient(to bottom, #5a5a5a, #3a3a3a);
    border-color: #666;
  }

  .control-button:active:not(:disabled) {
    background: linear-gradient(to bottom, #2a2a2a, #4a4a4a);
    box-shadow: 
      inset 0 2px 4px rgba(0,0,0,0.5),
      0 1px 1px rgba(0,0,0,0.3);
  }

  .control-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .control-button .icon {
    font-size: 20px;
  }

  .control-button .label {
    font-size: 10px;
    letter-spacing: 1px;
  }

  .control-button.play:not(:disabled) {
    border-color: #4a7c59;
    color: #7fbf7f;
  }

  .control-button.stop:not(:disabled) {
    border-color: #7c4a4a;
    color: #ff7f7f;
  }

  .request-editor {
    margin-top: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .request-input {
    width: 100%;
    height: 120px;
    background: #1a1a1a;
    border: 1px solid #444;
    border-radius: 4px;
    color: #c0c0c0;
    padding: 8px;
    font-family: monospace;
    font-size: 12px;
    resize: vertical;
  }

  .request-input:focus {
    outline: none;
    border-color: #666;
  }

  .send-button {
    background: linear-gradient(to bottom, #4a7c59, #2a5c39);
    border: 2px solid #5a8c69;
    border-radius: 4px;
    padding: 8px 16px;
    color: #7fbf7f;
    font-weight: bold;
    cursor: pointer;
    transition: all 0.1s ease;
    align-self: flex-end;
  }

  .send-button:hover {
    background: linear-gradient(to bottom, #5a8c69, #3a6c49);
    border-color: #6a9c79;
  }

  .send-button:active {
    background: linear-gradient(to bottom, #2a5c39, #4a7c59);
  }
</style>