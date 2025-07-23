<script>
  import { isPlaying, currentCassette, selectedCassettes } from '../stores/cassette.js';
  import { soundEffects } from '../stores/sound.js';
  import { onDestroy } from 'svelte';
  
  let reelRotation = 0;
  let animationFrame;
  
  $: if ($isPlaying) {
    startAnimation();
    soundEffects.startMotor();
  } else {
    stopAnimation();
    soundEffects.stopMotor();
  }
  
  function startAnimation() {
    if (animationFrame) return; // Already running
    animateFrame();
  }
  
  function stopAnimation() {
    if (animationFrame) {
      cancelAnimationFrame(animationFrame);
      animationFrame = null;
    }
    reelRotation = 0;
  }
  
  function animateFrame() {
    if (!$isPlaying) return;
    reelRotation += 2;
    animationFrame = requestAnimationFrame(animateFrame);
  }
  
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
  
  function pause() {
    isPlaying.set(false);
    soundEffects.playClick();
  }
  
  function record() {
    if ($selectedCassettes.size > 0) {
      isPlaying.set(true);
      soundEffects.playClick();
    }
  }
  
  onDestroy(() => {
    stopAnimation();
  });
</script>

<div class="tape-transport">
  <div class="transport-display">
    <!-- Tape reels visualization -->
    <div class="tape-reels">
      <div 
        class="reel left"
        class:spinning={$isPlaying}
        style="transform: rotate({reelRotation}deg)"
      >
        <div class="reel-hub"></div>
        <div class="reel-spokes"></div>
      </div>
      
      <div class="tape-window">
        <div class="tape" class:moving={$isPlaying}></div>
      </div>
      
      <div 
        class="reel right"
        class:spinning={$isPlaying}
        style="transform: rotate({-reelRotation}deg)"
      >
        <div class="reel-hub"></div>
        <div class="reel-spokes"></div>
      </div>
    </div>
    
    <!-- Counter display -->
    <div class="counter">
      <span class="counter-digits">0000</span>
    </div>
  </div>
  
  <!-- Transport buttons -->
  <div class="transport-buttons">
    <button 
      class="transport-btn record"
      class:active={$isPlaying && $selectedCassettes.size > 0}
      disabled={$selectedCassettes.size === 0}
      on:click={record}
    >
      <div class="btn-face">
        <span class="btn-icon">●</span>
        <span class="btn-label">REC</span>
      </div>
    </button>
    
    <button 
      class="transport-btn play"
      class:active={$isPlaying}
      disabled={!$currentCassette}
      on:click={play}
    >
      <div class="btn-face">
        <span class="btn-icon">▶</span>
        <span class="btn-label">PLAY</span>
      </div>
    </button>
    
    <button 
      class="transport-btn stop"
      disabled={!$isPlaying}
      on:click={stop}
    >
      <div class="btn-face">
        <span class="btn-icon">■</span>
        <span class="btn-label">STOP</span>
      </div>
    </button>
    
    <button 
      class="transport-btn pause"
      disabled={!$isPlaying}
      on:click={pause}
    >
      <div class="btn-face">
        <span class="btn-icon">⏸</span>
        <span class="btn-label">PAUSE</span>
      </div>
    </button>
  </div>
</div>

<style>
  .tape-transport {
    display: flex;
    flex-direction: column;
    gap: 15px;
  }
  
  /* Transport display */
  .transport-display {
    background: #0a0a0a;
    border: 2px solid #333;
    border-radius: 4px;
    padding: 15px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  
  /* Tape reels */
  .tape-reels {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    height: 60px;
  }
  
  .reel {
    width: 50px;
    height: 50px;
    background: #222;
    border: 2px solid #444;
    border-radius: 50%;
    position: relative;
    transition: transform 0.1s linear;
  }
  
  .reel-hub {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 15px;
    height: 15px;
    background: #111;
    border-radius: 50%;
  }
  
  .reel-spokes {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 40px;
    height: 2px;
    background: #333;
    transform: translate(-50%, -50%);
  }
  
  .reel-spokes::before,
  .reel-spokes::after {
    content: '';
    position: absolute;
    width: 100%;
    height: 100%;
    background: #333;
  }
  
  .reel-spokes::before {
    transform: rotate(60deg);
  }
  
  .reel-spokes::after {
    transform: rotate(120deg);
  }
  
  /* Tape window */
  .tape-window {
    flex: 1;
    height: 20px;
    background: #111;
    border: 1px solid #333;
    border-radius: 2px;
    position: relative;
    overflow: hidden;
  }
  
  .tape {
    position: absolute;
    top: 50%;
    left: -100%;
    right: -100%;
    height: 8px;
    background: repeating-linear-gradient(
      90deg,
      #444 0px,
      #444 2px,
      #333 2px,
      #333 4px
    );
    transform: translateY(-50%);
  }
  
  .tape.moving {
    animation: tape-move 2s linear infinite;
  }
  
  @keyframes tape-move {
    0% { transform: translateY(-50%) translateX(0); }
    100% { transform: translateY(-50%) translateX(4px); }
  }
  
  /* Counter */
  .counter {
    background: #000;
    border: 1px solid #222;
    border-radius: 2px;
    padding: 4px 8px;
    text-align: center;
  }
  
  .counter-digits {
    font-family: 'Share Tech Mono', monospace;
    font-size: 14px;
    color: #f80;
    text-shadow: 0 0 3px #f80;
    letter-spacing: 0.1em;
  }
  
  /* Transport buttons */
  .transport-buttons {
    display: flex;
    justify-content: center;
    gap: 8px;
  }
  
  .transport-btn {
    width: 60px;
    height: 50px;
    background: linear-gradient(to bottom, #4a4a4a, #3a3a3a);
    border: 2px solid #555;
    border-radius: 4px;
    cursor: pointer;
    padding: 0;
    transition: all 0.1s;
    box-shadow: 
      0 4px 8px rgba(0, 0, 0, 0.3),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
  }
  
  .btn-face {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    height: 100%;
  }
  
  .transport-btn:hover:not(:disabled) {
    background: linear-gradient(to bottom, #5a5a5a, #4a4a4a);
  }
  
  .transport-btn:active:not(:disabled) {
    transform: translateY(2px);
    box-shadow: 
      0 2px 4px rgba(0, 0, 0, 0.3),
      inset 0 2px 4px rgba(0, 0, 0, 0.5);
  }
  
  .transport-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  
  .transport-btn.active {
    box-shadow: 
      0 2px 4px rgba(0, 0, 0, 0.3),
      inset 0 2px 4px rgba(0, 0, 0, 0.5);
    transform: translateY(2px);
  }
  
  /* Button specific styles */
  .transport-btn.record .btn-icon {
    color: #f00;
    text-shadow: 0 0 4px #f00;
  }
  
  .transport-btn.record.active .btn-icon {
    color: #f00;
    text-shadow: 0 0 8px #f00;
    animation: record-pulse 1s infinite;
  }
  
  @keyframes record-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.6; }
  }
  
  .transport-btn.play .btn-icon {
    color: #0f0;
  }
  
  .transport-btn.play.active .btn-icon {
    text-shadow: 0 0 8px #0f0;
  }
  
  .btn-icon {
    font-size: 18px;
    color: #ccc;
  }
  
  .btn-label {
    font-size: 9px;
    color: #888;
    letter-spacing: 0.1em;
  }
</style>