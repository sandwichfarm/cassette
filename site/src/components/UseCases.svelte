<script>
  import { onMount } from 'svelte'
  
  let usecasesElement
  let visible = false
  let selectedCase = 0
  let autoplay = true
  
  onMount(() => {
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          visible = true
          startAutoplay()
        } else {
          autoplay = false
        }
      },
      { threshold: 0.1 }
    )
    
    if (usecasesElement) observer.observe(usecasesElement)
    
    return () => {
      if (usecasesElement) observer.unobserve(usecasesElement)
      autoplay = false
    }
  })
  
  function startAutoplay() {
    const interval = setInterval(() => {
      if (!autoplay) {
        clearInterval(interval)
        return
      }
      selectedCase = (selectedCase + 1) % cassettes.length
    }, 4000)
  }
  
  function selectCase(index) {
    selectedCase = index
    autoplay = false // Stop autoplay when user interacts
  }
  
  const cassettes = [
    {
      title: 'ARCHIVE MIX',
      subtitle: 'Preserve the Past',
      description: 'Your digital time capsule. Capture moments from the social web and seal them in portable WebAssembly containers. Perfect for backing up conversations, preserving social graphs, or creating historical snapshots.',
      color: 'from-cassette-red to-cassette-orange',
      accent: '#ef4444',
      details: ['Immutable storage', 'Social graph backup', 'Historical preservation'],
      code: 'cassette record archive.json --name "my-archive"'
    },
    {
      title: 'OFFLINE HITS',
      subtitle: 'Disconnect to Connect',
      description: 'Build apps that work anywhere, anytime. Query Nostr events without internet connectivity while maintaining full protocol compatibility. Your users stay connected even when they\'re not.',
      color: 'from-cassette-blue to-cassette-purple',
      accent: '#3b82f6',
      details: ['Zero network dependency', 'Full NIP-01 support', 'Mobile-first design'],
      code: 'cassette play offline.wasm --filter=\'{"kinds": [1]}\''
    },
    {
      title: 'TEST SUITE',
      subtitle: 'Deterministic Beats',
      description: 'Reproducible test environments for Nostr development. Same events, same behavior, every single test run. No more flaky tests or network dependencies in your CI/CD pipeline.',
      color: 'from-cassette-yellow to-cassette-orange',
      accent: '#fbbf24',
      details: ['Deterministic testing', 'CI/CD friendly', 'Isolated environments'],
      code: 'cassette record test-data.json --name "test-fixture"'
    },
    {
      title: 'MIXTAPE SHARE',
      subtitle: 'Curated Collections',
      description: 'Share curated event collections like mixtapes. Distribute reading lists, conversation threads, or topic compilations as single, portable files. Social curation meets digital distribution.',
      color: 'from-cassette-purple to-cassette-blue',
      accent: '#9333ea',
      details: ['Content curation', 'Easy distribution', 'Social discovery'],
      code: 'cassette dub topic1.wasm topic2.wasm --output mixtape.wasm'
    },
    {
      title: 'EDGE STATION',
      subtitle: 'Global Distribution',
      description: 'Deploy Nostr relays to the edge using WebAssembly. Serve events closer to your users with minimal infrastructure. Scale globally without the complexity.',
      color: 'from-cassette-orange to-cassette-yellow',
      accent: '#f97316',
      details: ['Edge deployment', 'Global distribution', 'Minimal infrastructure'],
      code: 'cassette cast events.wasm --relays wss://edge.relay'
    },
    {
      title: 'PRIVACY VAULT',
      subtitle: 'Keep It Local',
      description: 'Store sensitive events locally while maintaining standard query interfaces. No external relays, no data leaks, no compromises. Your data stays yours, period.',
      color: 'from-cassette-red to-cassette-purple',
      accent: '#ef4444',
      details: ['Local-first', 'Zero external calls', 'Privacy by design'],
      code: 'cassette record private.json --output ~/.cassettes/vault.wasm'
    }
  ]
</script>

<section bind:this={usecasesElement} class="py-20 bg-gradient-to-b from-cassette-black to-gray-900 text-white">
  <div class="container mx-auto px-6">
    <!-- Section Header -->
    <div class={`text-center mb-16 transition-all duration-700 ${visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-10'}`}>
      <div class="text-cassette-yellow text-sm font-mono tracking-widest mb-4">
        ◀ CASSETTE COLLECTION ▶
      </div>
      <h2 class="text-5xl md:text-6xl font-black mb-6 bg-gradient-to-r from-cassette-orange to-cassette-yellow bg-clip-text text-transparent">
        USE CASES
      </h2>
      <p class="text-xl text-gray-300 max-w-3xl mx-auto leading-relaxed">
        Each use case is a different kind of mixtape. Click to explore how others are 
        mixing, sharing, and playing back their slice of the social web.
      </p>
    </div>

    <!-- Main Interactive Area -->
    <div class={`transition-all duration-1000 ${visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-10'}`}>
      <div class="grid lg:grid-cols-2 gap-12 items-center">
        
        <!-- Cassette Player Interface -->
        <div class="order-2 lg:order-1">
          <!-- Current Playing Display -->
          <div class="bg-cassette-black bg-opacity-50 rounded-2xl p-8 border border-cassette-silver border-opacity-20 mb-8">
            <div class="flex items-center justify-between mb-6">
              <div class="text-cassette-yellow font-mono text-sm tracking-widest">
                NOW PLAYING
              </div>
              <div class="flex gap-2">
                <div class="w-3 h-3 bg-cassette-red rounded-full animate-pulse"></div>
                <div class="w-3 h-3 bg-cassette-yellow rounded-full animate-pulse" style="animation-delay: 0.5s"></div>
                <div class="w-3 h-3 bg-cassette-purple rounded-full animate-pulse" style="animation-delay: 1s"></div>
              </div>
            </div>
            
            <h3 class={`text-3xl font-black mb-2 bg-gradient-to-r ${cassettes[selectedCase].color} bg-clip-text text-transparent`}>
              {cassettes[selectedCase].title}
            </h3>
            <p class="text-cassette-silver text-lg mb-4 font-mono">
              {cassettes[selectedCase].subtitle}
            </p>
            <p class="text-gray-300 mb-6 leading-relaxed">
              {cassettes[selectedCase].description}
            </p>
            
            <!-- Feature List -->
            <div class="flex flex-wrap gap-2 mb-6">
              {#each cassettes[selectedCase].details as detail}
                <span class={`px-3 py-1 bg-gradient-to-r ${cassettes[selectedCase].color} bg-opacity-20 rounded-full text-sm font-mono border border-opacity-30`} style="border-color: {cassettes[selectedCase].accent}">
                  {detail}
                </span>
              {/each}
            </div>
            
            <!-- Code Example -->
            <div class="bg-cassette-black rounded-lg p-4 font-mono text-sm border border-cassette-silver border-opacity-20">
              <div class="text-cassette-yellow mb-2">$ {cassettes[selectedCase].code}</div>
              <div class="text-cassette-silver opacity-60">⚡ Compiling WASM module...</div>
              <div class="text-cassette-silver opacity-60">✅ Ready to play</div>
            </div>
          </div>
        </div>

        <!-- Cassette Collection -->
        <div class="order-1 lg:order-2">
          <div class="grid grid-cols-2 md:grid-cols-3 gap-4">
            {#each cassettes as cassette, i}
              <button
                on:click={() => selectCase(i)}
                class={`relative group transition-all duration-300 transform hover:scale-105 hover:z-10 ${
                  selectedCase === i ? 'scale-110 z-20' : ''
                }`}
              >
                <!-- Cassette Tape -->
                <div class={`w-full aspect-[3/2] bg-gradient-to-b from-cassette-silver to-gray-600 rounded-lg relative shadow-xl transition-all duration-300 ${
                  selectedCase === i ? 'shadow-2xl ring-4 ring-opacity-50' : 'hover:shadow-2xl'
                }`} style="ring-color: {cassette.accent}">
                  
                  <!-- Selection glow -->
                  {#if selectedCase === i}
                    <div class="absolute inset-0 rounded-lg animate-pulse" style="box-shadow: 0 0 30px {cassette.accent}40"></div>
                  {/if}
                  
                  <!-- Tape reels -->
                  <div class="absolute top-2 left-3 w-4 h-4 bg-cassette-black rounded-full"></div>
                  <div class="absolute top-2 right-3 w-4 h-4 bg-cassette-black rounded-full"></div>
                  
                  <!-- Spinning reels when selected -->
                  <div class={`absolute top-2.5 left-3.5 w-3 h-3 bg-gradient-to-r ${cassette.color} rounded-full transition-all duration-300 ${
                    selectedCase === i ? 'animate-spin' : ''
                  }`}></div>
                  <div class={`absolute top-2.5 right-3.5 w-3 h-3 bg-gradient-to-r ${cassette.color} rounded-full transition-all duration-300 ${
                    selectedCase === i ? 'animate-spin' : ''
                  }`}></div>
                  
                  <!-- Tape window -->
                  <div class={`absolute top-8 left-4 right-4 h-2 bg-gradient-to-r ${cassette.color} rounded-full opacity-80`}></div>
                  
                  <!-- Label -->
                  <div class="absolute bottom-1 left-1 right-1 h-3 bg-white rounded text-xs flex items-center justify-center text-black font-mono font-bold overflow-hidden">
                    <span class="truncate px-1">{cassette.title}</span>
                  </div>
                  
                  <!-- Hover overlay -->
                  <div class="absolute inset-0 bg-gradient-to-t from-black via-transparent to-transparent opacity-0 group-hover:opacity-50 transition-opacity duration-300 rounded-lg"></div>
                </div>
                
                <!-- Title below cassette -->
                <div class="mt-2 text-center">
                  <div class={`text-xs font-mono font-bold transition-colors duration-300 ${
                    selectedCase === i ? 'text-cassette-yellow' : 'text-gray-400 group-hover:text-white'
                  }`}>
                    {cassette.title}
                  </div>
                </div>
              </button>
            {/each}
          </div>
          
          <!-- Player Controls -->
          <div class="flex justify-center mt-8 gap-4">
            <button 
              on:click={() => selectCase((selectedCase - 1 + cassettes.length) % cassettes.length)}
              class="cassette-control-btn"
            >
              ⏮️
            </button>
            <button 
              on:click={() => autoplay = !autoplay}
              class="cassette-control-btn"
            >
              {autoplay ? '⏸️' : '▶️'}
            </button>
            <button 
              on:click={() => selectCase((selectedCase + 1) % cassettes.length)}
              class="cassette-control-btn"
            >
              ⏭️
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Call to Action -->
    <div class={`mt-20 text-center transition-all duration-1000 delay-500 ${visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-10'}`}>
      <div class="bg-gradient-to-r from-cassette-orange to-cassette-red rounded-2xl p-8 border border-cassette-orange border-opacity-30">
        <h3 class="text-3xl font-bold mb-4 text-white">
          Ready to Mix Your Own?
        </h3>
        <p class="text-gray-100 mb-6 max-w-2xl mx-auto opacity-90">
          Start experimenting with your own social data cassettes. 
          The only limit is your imagination.
        </p>
        <a href="#quickstart" class="inline-block px-8 py-4 bg-white text-cassette-orange rounded-lg font-bold text-lg font-mono tracking-wide hover:shadow-2xl transform hover:scale-105 transition-all duration-300">
          <span class="mr-2">🎛️</span>
          START MIXING
        </a>
      </div>
    </div>
  </div>
</section>

<style>
  .cassette-control-btn {
    width: 3rem;
    height: 3rem;
    background: linear-gradient(145deg, #94a3b8, #64748b);
    border: 2px solid #475569;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.2rem;
    transition: all 0.2s ease;
    box-shadow: 
      inset 2px 2px 4px rgba(255, 255, 255, 0.1),
      inset -2px -2px 4px rgba(0, 0, 0, 0.2),
      0 4px 8px rgba(0, 0, 0, 0.3);
  }
  
  .cassette-control-btn:hover {
    transform: scale(1.1);
    box-shadow: 
      inset 2px 2px 4px rgba(255, 255, 255, 0.2),
      inset -2px -2px 4px rgba(0, 0, 0, 0.3),
      0 6px 12px rgba(0, 0, 0, 0.4);
  }
  
  .cassette-control-btn:active {
    transform: scale(1.05) translateY(1px);
    box-shadow: 
      inset 2px 2px 4px rgba(0, 0, 0, 0.2),
      inset -2px -2px 4px rgba(255, 255, 255, 0.05),
      0 2px 4px rgba(0, 0, 0, 0.2);
  }
</style>