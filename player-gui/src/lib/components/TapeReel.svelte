<script>
  export let spinning = false;
  export let side = 'left';
  
  // Tape reel size changes based on side to simulate tape movement
  $: reelSize = side === 'left' ? (spinning ? 'small' : 'large') : (spinning ? 'large' : 'small');
</script>

<div class="tape-reel-container">
  <div class="tape-reel {spinning ? 'spinning' : ''} {reelSize}">
    <div class="hub">
      <div class="hole"></div>
      <div class="hole"></div>
      <div class="hole"></div>
    </div>
    <div class="tape-visible"></div>
  </div>
  <div class="spindle"></div>
</div>

<style>
  .tape-reel-container {
    position: relative;
    width: 120px;
    height: 120px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .spindle {
    position: absolute;
    width: 20px;
    height: 20px;
    background: #666;
    border-radius: 50%;
    box-shadow: 
      inset 0 1px 2px rgba(0,0,0,0.5),
      0 1px 1px rgba(255,255,255,0.1);
  }

  .tape-reel {
    width: 100px;
    height: 100px;
    border-radius: 50%;
    background: #8b4513;
    position: relative;
    box-shadow: 
      0 2px 4px rgba(0,0,0,0.3),
      inset 0 -2px 4px rgba(0,0,0,0.2);
    transition: all 0.3s ease;
  }

  .tape-reel.spinning {
    animation: spin 2s linear infinite;
    will-change: transform; /* Optimize for GPU acceleration */
  }

  .tape-reel.small {
    width: 80px;
    height: 80px;
  }

  .tape-reel.large {
    width: 110px;
    height: 110px;
  }

  .hub {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 40px;
    height: 40px;
    background: #333;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }

  .hole {
    width: 8px;
    height: 8px;
    background: #000;
    border-radius: 50%;
  }

  .tape-visible {
    position: absolute;
    top: 5px;
    left: 5px;
    right: 5px;
    bottom: 5px;
    border-radius: 50%;
    background: repeating-radial-gradient(
      circle at center,
      #6b3410 0px,
      #8b4513 2px,
      #6b3410 4px
    );
    opacity: 0.8;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
</style>