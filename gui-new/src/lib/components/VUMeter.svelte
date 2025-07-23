<script>
  import { onMount, onDestroy } from 'svelte';
  
  export let active = false;
  export let channel = 'L';
  
  let level = 0;
  let peakLevel = 0;
  let animationFrame;
  
  function animate() {
    if (active) {
      // Throttle animation to 30fps instead of 60fps
      setTimeout(() => {
        // Simulate audio levels with random values
        level = Math.random() * 100;
        
        // Peak hold behavior
        if (level > peakLevel) {
          peakLevel = level;
        } else {
          peakLevel = Math.max(0, peakLevel - 2);
        }
        
        animationFrame = requestAnimationFrame(animate);
      }, 33); // ~30fps
    } else {
      level = 0;
      peakLevel = 0;
    }
  }
  
  $: if (active) {
    animate();
  } else {
    if (animationFrame) {
      cancelAnimationFrame(animationFrame);
    }
    level = 0;
    peakLevel = 0;
  }
  
  onDestroy(() => {
    if (animationFrame) {
      cancelAnimationFrame(animationFrame);
    }
  });
</script>

<div class="vu-meter">
  <div class="channel-label">{channel}</div>
  <div class="meter-container">
    <div class="meter-background">
      <!-- Scale markings -->
      <div class="scale">
        <div class="mark" style="left: 0%">-∞</div>
        <div class="mark" style="left: 50%">-10</div>
        <div class="mark" style="left: 70%">-5</div>
        <div class="mark" style="left: 85%">0</div>
        <div class="mark red" style="left: 95%">+3</div>
      </div>
      
      <!-- Level bar -->
      <div class="level-bar" style="width: {level}%">
        <div class="level-gradient"></div>
      </div>
      
      <!-- Peak indicator -->
      <div class="peak-indicator" style="left: {peakLevel}%"></div>
    </div>
  </div>
</div>

<style>
  .vu-meter {
    width: 200px;
  }

  .channel-label {
    text-align: center;
    color: #c0c0c0;
    font-size: 12px;
    font-weight: bold;
    margin-bottom: 4px;
  }

  .meter-container {
    background: #1a1a1a;
    border: 1px solid #444;
    border-radius: 2px;
    padding: 4px;
    box-shadow: inset 0 1px 3px rgba(0,0,0,0.5);
  }

  .meter-background {
    position: relative;
    height: 20px;
    background: #0a0a0a;
    border-radius: 2px;
    overflow: hidden;
  }

  .scale {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 8px;
    font-size: 8px;
    color: #666;
  }

  .mark {
    position: absolute;
    transform: translateX(-50%);
    top: 0;
  }

  .mark.red {
    color: #ff3333;
  }

  .level-bar {
    position: absolute;
    bottom: 0;
    left: 0;
    height: 12px;
    transition: width 50ms ease-out;
    overflow: hidden;
  }

  .level-gradient {
    width: 200px;
    height: 100%;
    background: linear-gradient(
      to right,
      #00ff00 0%,
      #00ff00 70%,
      #ffff00 85%,
      #ff0000 95%
    );
  }

  .peak-indicator {
    position: absolute;
    bottom: 0;
    width: 2px;
    height: 12px;
    background: #fff;
    transition: left 50ms ease-out;
    box-shadow: 0 0 2px rgba(255,255,255,0.8);
  }
</style>