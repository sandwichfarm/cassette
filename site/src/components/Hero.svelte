<script>
  import { onMount } from 'svelte'
  
  let visible = false
  let cassettePlaying = false
  let cassetteClicked = false
  let cassetteElement = null
  
  onMount(() => {
    visible = true
    setTimeout(() => {
      cassettePlaying = true
    }, 800)
  })
  
  function handleCassetteClick() {
    if (cassetteClicked) return // Prevent multiple clicks
    
    cassetteClicked = true
    // Don't reset - cassette stays hidden after jetting off
  }
</script>

<section class="relative min-h-screen flex items-center justify-center bg-gradient-to-br from-cassette-black via-cassette-dark to-gray-900 text-white overflow-hidden">
  
  <!-- Retro-wave perspective grid floor -->
  <div class="absolute inset-0 pointer-events-none overflow-hidden">
    <div class="retro-perspective-container">
      <svg class="w-full h-full" viewBox="0 0 1200 800" preserveAspectRatio="none">
        <defs>
          <!-- Grid line gradients -->
          <linearGradient id="grid-line-gradient" x1="0%" y1="0%" x2="0%" y2="100%">
            <stop offset="0%" style="stop-color:#3b82f6;stop-opacity:0"/>
            <stop offset="40%" style="stop-color:#9333ea;stop-opacity:0.6"/>
            <stop offset="100%" style="stop-color:#f97316;stop-opacity:0.8"/>
          </linearGradient>
          
          <!-- Horizontal perspective lines -->
          <linearGradient id="horizon-line-gradient" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" style="stop-color:#9333ea;stop-opacity:0"/>
            <stop offset="50%" style="stop-color:#f97316;stop-opacity:0.6"/>
            <stop offset="100%" style="stop-color:#9333ea;stop-opacity:0"/>
          </linearGradient>
        </defs>
        
        <!-- Perspective floor grid -->
        <g class="grid-floor">
          <!-- Horizontal lines (getting closer together as they recede) -->
          <line x1="0" y1="800" x2="1200" y2="800" stroke="url(#grid-line-gradient)" stroke-width="2" opacity="0.8"/>
          <line x1="0" y1="760" x2="1200" y2="760" stroke="url(#grid-line-gradient)" stroke-width="2" opacity="0.7"/>
          <line x1="0" y1="720" x2="1200" y2="720" stroke="url(#grid-line-gradient)" stroke-width="1.5" opacity="0.6"/>
          <line x1="0" y1="680" x2="1200" y2="680" stroke="url(#grid-line-gradient)" stroke-width="1.5" opacity="0.5"/>
          <line x1="0" y1="640" x2="1200" y2="640" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.4"/>
          <line x1="0" y1="600" x2="1200" y2="600" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.35"/>
          <line x1="0" y1="565" x2="1200" y2="565" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.3"/>
          <line x1="0" y1="535" x2="1200" y2="535" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.25"/>
          <line x1="0" y1="510" x2="1200" y2="510" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.2"/>
          <line x1="0" y1="490" x2="1200" y2="490" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.15"/>
          <line x1="0" y1="475" x2="1200" y2="475" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.1"/>
          <line x1="0" y1="465" x2="1200" y2="465" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.08"/>
          <line x1="0" y1="458" x2="1200" y2="458" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.06"/>
          <line x1="0" y1="453" x2="1200" y2="453" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.04"/>
          <line x1="0" y1="450" x2="1200" y2="450" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.02"/>
          
          <!-- Vertical perspective lines (converging to center) -->
          <line x1="100" y1="800" x2="580" y2="450" stroke="url(#grid-line-gradient)" stroke-width="1.5" opacity="0.4"/>
          <line x1="200" y1="800" x2="590" y2="450" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.3"/>
          <line x1="300" y1="800" x2="595" y2="450" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.3"/>
          <line x1="400" y1="800" x2="598" y2="450" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.3"/>
          <line x1="500" y1="800" x2="600" y2="450" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.3"/>
          
          <!-- Center line -->
          <line x1="600" y1="800" x2="600" y2="450" stroke="url(#grid-line-gradient)" stroke-width="2" opacity="0.5"/>
          
          <!-- Right side -->
          <line x1="700" y1="800" x2="600" y2="450" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.3"/>
          <line x1="800" y1="800" x2="602" y2="450" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.3"/>
          <line x1="900" y1="800" x2="605" y2="450" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.3"/>
          <line x1="1000" y1="800" x2="610" y2="450" stroke="url(#grid-line-gradient)" stroke-width="1" opacity="0.3"/>
          <line x1="1100" y1="800" x2="620" y2="450" stroke="url(#grid-line-gradient)" stroke-width="1.5" opacity="0.4"/>
        </g>
        
        <!-- Horizon glow -->
        <line x1="0" y1="450" x2="1200" y2="450" stroke="url(#horizon-line-gradient)" stroke-width="3" opacity="0.7">
          <animate attributeName="opacity" values="0.7;1;0.7" dur="3s" repeatCount="indefinite"/>
        </line>
        
      </svg>
    </div>
  </div>
  
  <!-- Floating cassette tape visual -->
  <div class="absolute top-20 right-10 opacity-30 hover:opacity-100 transition-all duration-500 group cursor-pointer" 
       on:click={handleCassetteClick}>
    <div bind:this={cassetteElement}
         class={`cassette-container transform rotate-12 group-hover:rotate-6 group-hover:scale-110 transition-all duration-500 ${
           cassetteClicked ? 'cassette-jet-off' : ''
         }`}>
      <!-- Main cassette body -->
      <div class="w-40 h-24 bg-gradient-to-b from-cassette-silver to-gray-600 rounded-lg relative shadow-2xl group-hover:shadow-3xl transition-all duration-500">
        <!-- Tape reels -->
        <div class="absolute top-3 left-4 w-6 h-6 bg-cassette-black rounded-full"></div>
        <div class="absolute top-3 right-4 w-6 h-6 bg-cassette-black rounded-full"></div>
        <!-- Spinning reels when playing -->
        <div class={`absolute top-4 left-5 w-4 h-4 bg-gradient-to-r from-cassette-orange to-cassette-yellow rounded-full ${cassettePlaying || 'group-hover' ? 'animate-cassette-spin' : ''}`}></div>
        <div class={`absolute top-4 right-5 w-4 h-4 bg-gradient-to-r from-cassette-orange to-cassette-yellow rounded-full ${cassettePlaying || 'group-hover' ? 'animate-cassette-spin' : ''}`}></div>
        
        <!-- Tape window -->
        <div class="absolute top-12 left-8 right-8 h-3 bg-gradient-to-r from-cassette-orange via-cassette-yellow to-cassette-orange rounded-full opacity-80 group-hover:opacity-100 transition-opacity duration-300"></div>
        
        <!-- Label -->
        <div class="absolute bottom-2 left-2 right-2 h-3 bg-white rounded text-xs flex items-center justify-center text-black font-mono">
          MY RARE NOTES
        </div>
        
        <!-- Hover glow effect -->
        <div class="absolute inset-0 rounded-lg opacity-0 group-hover:opacity-100 transition-opacity duration-500" style="box-shadow: 0 0 30px rgba(249, 115, 22, 0.6), 0 0 60px rgba(249, 115, 22, 0.3)"></div>
      </div>
    </div>
  </div>
  
  <div class="container mx-auto px-6 z-10 text-center relative">
    <div class={`transition-all duration-1000 transform ${visible ? 'translate-y-0 opacity-100' : 'translate-y-10 opacity-0'}`}>
      <!-- Main cassette logo/title -->
      <div class="mb-8 flex justify-center">
        <div class="relative">
          <h1 class="text-6xl md:text-8xl font-black tracking-wider bg-gradient-to-r from-cassette-orange via-cassette-yellow to-cassette-red bg-clip-text text-transparent">
            CASSETTE
          </h1>
          <!-- Retro underline -->
          <div class="absolute -bottom-2 left-0 right-0 h-1 bg-gradient-to-r from-cassette-purple to-cassette-blue animate-retro-glow rounded-full"></div>
        </div>
      </div>
      
      <!-- Subtitle with retro styling -->
      <div class="text-center mb-8 font-mono">
        <p class="text-sm md:text-base text-cassette-yellow tracking-widest mb-2">
          ▶ CONCEPT.INITIALIZED ◀
        </p>
        <p class="text-xl md:text-3xl mb-4 text-gray-300 max-w-4xl mx-auto leading-relaxed">
          What if notes could be <span class="text-cassette-orange font-bold">recorded</span>, 
          <span class="text-cassette-yellow font-bold">distributed</span>, and 
          <span class="text-cassette-purple font-bold">replayed</span>?
        </p>
        <p class="text-lg text-gray-400 max-w-2xl mx-auto">
          Store social data in portable WebAssembly modules. Share, replay, and query them anywhere. 
          It's not just technology—it's a new way to think about digital preservation.
        </p>
      </div>
      
      <!-- Control buttons styled like cassette player -->
      <div class="flex flex-col sm:flex-row gap-4 justify-center mb-12">
        <a href="#concept" class="cassette-button bg-gradient-to-r from-cassette-orange to-cassette-red">
          <span class="text-2xl mr-2">▶</span>
          EXPLORE THE CONCEPT
        </a>
        <a href="https://github.com/dskvr/cassette" target="_blank" rel="noopener noreferrer" class="cassette-button bg-gradient-to-r from-cassette-purple to-cassette-blue">
          <span class="text-2xl mr-2">⚡</span>
          VIEW SOURCE
        </a>
      </div>
      
      <!-- Retro status indicators -->
      <div class="flex flex-wrap gap-8 justify-center text-sm text-cassette-chrome font-mono">
        <div class="flex items-center gap-2 bg-black bg-opacity-30 px-4 py-2 rounded border border-cassette-silver border-opacity-30">
          <div class="w-2 h-2 bg-cassette-red rounded-full animate-pulse"></div>
          WASM.READY
        </div>
        <div class="flex items-center gap-2 bg-black bg-opacity-30 px-4 py-2 rounded border border-cassette-silver border-opacity-30">
          <div class="w-2 h-2 bg-cassette-yellow rounded-full animate-pulse"></div>
          NOSTR.COMPATIBLE
        </div>
        <div class="flex items-center gap-2 bg-black bg-opacity-30 px-4 py-2 rounded border border-cassette-silver border-opacity-30">
          <div class="w-2 h-2 bg-cassette-purple rounded-full animate-pulse"></div>
          PORTABLE.EVERYWHERE
        </div>
      </div>
    </div>
  </div>
  
  <!-- Retro scroll indicator -->
  <div class="absolute bottom-10 left-1/2 transform -translate-x-1/2">
    <div class="text-center">
      <div class="text-cassette-yellow text-xs font-mono mb-2 tracking-widest">SCROLL TO CONTINUE</div>
      <div class="w-6 h-6 border-2 border-cassette-yellow rounded animate-bounce mx-auto">
        <div class="w-1 h-1 bg-cassette-yellow rounded-full mx-auto mt-2 animate-pulse"></div>
      </div>
    </div>
  </div>
</section>

<style>
  .bg-retro-grid {
    background-image: 
      linear-gradient(rgba(249, 115, 22, 0.2) 1px, transparent 1px),
      linear-gradient(90deg, rgba(249, 115, 22, 0.2) 1px, transparent 1px);
    background-size: 40px 40px;
    animation: gridMove 20s linear infinite;
  }
  
  @keyframes gridMove {
    0% { transform: translate(0, 0); }
    100% { transform: translate(40px, 40px); }
  }
  
  .cassette-button {
    padding: 2rem 2rem;
    border-radius: 0.5rem;
    font-weight: bold;
    font-size: 1.125rem;
    font-family: ui-monospace, SFMono-Regular, monospace;
    letter-spacing: 0.025em;
    border: 2px solid rgba(255, 255, 255, 0.5);
    transition: all 0.3s ease;
    transform: scale(1);
    box-shadow: 
      0 0 20px rgba(0, 0, 0, 0.5),
      inset 0 2px 4px rgba(255, 255, 255, 0.2);
  }
  
  .cassette-button:hover {
    transform: scale(1.05);
    box-shadow: 
      0 25px 50px -12px rgba(0, 0, 0, 0.25),
      0 0 30px rgba(249, 115, 22, 0.5),
      inset 0 2px 4px rgba(255, 255, 255, 0.3);
  }
  
  /* Cassette jet-off animation */
  .cassette-jet-off {
    animation: cassetteJetOff 1.2s cubic-bezier(0.68, -0.55, 0.265, 1.55) forwards;
  }
  
  @keyframes cassetteJetOff {
    0% {
      transform: rotate(12deg) scale(1);
    }
    15% {
      transform: rotate(-5deg) scale(1.1);
    }
    100% {
      transform: rotate(-45deg) scale(0.3) translate(-150vw, -50vh);
      opacity: 0;
    }
  }
  
  /* Tape flow animation */
  @keyframes tapeFlow {
    0% {
      transform: translateX(-20px);
      opacity: 0;
    }
    10% {
      opacity: 0.4;
    }
    90% {
      opacity: 0.4;
    }
    100% {
      transform: translateX(100vw);
      opacity: 0;
    }
  }
  
  .animate-tape-flow {
    animation: tapeFlow 8s linear infinite;
  }
</style>