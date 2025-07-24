<script>
  import { onMount } from 'svelte'
  
  let conceptElement
  let visible = false
  let currentStep = 0
  
  onMount(() => {
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          visible = true
          // Animate through steps
          setTimeout(() => {
            const interval = setInterval(() => {
              currentStep++
              if (currentStep >= conceptSteps.length) {
                clearInterval(interval)
              }
            }, 1000)
          }, 500)
        }
      },
      { threshold: 0.1 }
    )
    
    if (conceptElement) observer.observe(conceptElement)
    
    return () => {
      if (conceptElement) observer.unobserve(conceptElement)
    }
  })
  
  const conceptSteps = [
    {
      step: "01",
      title: "RECORD",
      subtitle: "Capture the Moment",
      description: "Like pressing record on a cassette player, capture Nostr events from relays, files, or streams into a structured format.",
      visual: "⏺️",
      color: "from-cassette-red to-cassette-orange",
      details: "Your social data becomes a tangible artifact, ready to be preserved and shared."
    },
    {
      step: "02", 
      title: "PACKAGE",
      subtitle: "Seal in WebAssembly",
      description: "Transform captured events into a portable WebAssembly module—your own personal cassette tape of social data.",
      visual: "📦",
      color: "from-cassette-orange to-cassette-yellow",
      details: "Each cassette is self-contained, including the query engine and all the data you need."
    },
    {
      step: "03",
      title: "PLAY",
      subtitle: "Query Anywhere",
      description: "Insert your cassette into any environment—browsers, servers, mobile apps—and query like it's a relay.",
      visual: "▶️",
      color: "from-cassette-purple to-cassette-blue", 
      details: "Standard Nostr protocols work seamlessly. Your tools don't need to change."
    },
    {
      step: "04",
      title: "SHARE",
      subtitle: "Distribute the Experience",
      description: "Pass cassettes around like mixtapes. Each one contains a complete, reproducible slice of the social web.",
      visual: "🔄",
      color: "from-cassette-blue to-cassette-purple",
      details: "Perfect for testing, research, privacy, or just preserving digital memories."
    }
  ]
</script>

<section id="concept" bind:this={conceptElement} class="py-20 bg-gradient-to-b from-gray-900 to-cassette-black text-white">
  <div class="container mx-auto px-6">
    <!-- Section header -->
    <div class={`text-center mb-20 transition-all duration-700 ${visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-10'}`}>
      <div class="text-cassette-yellow text-sm font-mono tracking-widest mb-4">
        ◀ THE CONCEPT UNFOLDS ▶
      </div>
      <h2 class="text-5xl md:text-6xl font-black mb-6 bg-gradient-to-r from-cassette-orange to-cassette-yellow bg-clip-text text-transparent">
        HOW IT WORKS
      </h2>
      <p class="text-xl text-gray-300 max-w-3xl mx-auto leading-relaxed">
        Think of it like recording, packaging, and sharing social media moments—
        but with the permanence and portability of physical media.
      </p>
    </div>
    
    <!-- Concept steps -->
    <div class="space-y-16">
      {#each conceptSteps as concept, i}
        <div 
          class={`flex flex-col lg:flex-row items-center gap-12 transition-all duration-1000 ${
            visible && currentStep > i ? 'opacity-100 translate-x-0' : 'opacity-30 translate-x-10'
          } ${i % 2 === 1 ? 'lg:flex-row-reverse' : ''}`}
        >
          <!-- Visual element -->
          <div class="lg:w-1/3 flex justify-center">
            <div class={`relative group ${visible && currentStep > i ? 'animate-pulse' : ''}`}>
              <!-- Cassette tape representation -->
              <div class="w-48 h-32 bg-gradient-to-b from-cassette-silver to-gray-600 rounded-lg relative shadow-2xl transform group-hover:scale-105 transition-transform duration-300">
                <!-- Step number -->
                <div class={`absolute -top-4 -left-4 w-12 h-12 bg-gradient-to-r ${concept.color} rounded-full flex items-center justify-center text-white font-bold text-lg shadow-lg`}>
                  {concept.step}
                </div>
                
                <!-- Main visual -->
                <div class="absolute inset-0 flex items-center justify-center text-6xl">
                  {concept.visual}
                </div>
                
                <!-- Cassette details -->
                <div class="absolute bottom-2 left-2 right-2 h-4 bg-white rounded text-xs flex items-center justify-center text-black font-mono">
                  {concept.title}.WASM
                </div>
              </div>
            </div>
          </div>
          
          <!-- Content -->
          <div class="lg:w-2/3 text-center lg:text-left">
            <div class="mb-4">
              <div class={`inline-block px-4 py-2 bg-gradient-to-r ${concept.color} rounded-full text-black font-mono font-bold text-sm tracking-wider mb-4`}>
                {concept.subtitle}
              </div>
            </div>
            
            <h3 class="text-4xl md:text-5xl font-black mb-4 text-white">
              {concept.title}
            </h3>
            
            <p class="text-xl text-gray-300 mb-6 leading-relaxed">
              {concept.description}
            </p>
            
            <p class="text-lg text-gray-400 italic">
              {concept.details}
            </p>
            
            <!-- Progress indicator -->
            <div class="mt-8 flex gap-2">
              {#each conceptSteps as _, stepIndex}
                <div class={`h-1 flex-1 rounded transition-all duration-500 ${
                  stepIndex <= i && currentStep > i ? `bg-gradient-to-r ${concept.color}` : 'bg-gray-700'
                }`}></div>
              {/each}
            </div>
          </div>
        </div>
      {/each}
    </div>
    
    <!-- Call to action -->
    <div class={`text-center mt-20 transition-all duration-1000 ${visible && currentStep >= conceptSteps.length ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-10'}`}>
      <div class="bg-cassette-black bg-opacity-50 rounded-xl p-8 border border-cassette-orange border-opacity-30">
        <h3 class="text-3xl font-bold mb-4 text-cassette-yellow">
          Ready to start recording?
        </h3>
        <p class="text-gray-300 mb-6 max-w-2xl mx-auto">
          The future of social data is portable, private, and permanent. 
          Join us in exploring what becomes possible when we think beyond the relay.
        </p>
        <a href="#quickstart" class="inline-block px-8 py-4 bg-gradient-to-r from-cassette-orange to-cassette-red rounded-lg font-bold text-lg font-mono tracking-wide hover:shadow-2xl transform hover:scale-105 transition-all duration-300">
          <span class="mr-2">🎵</span>
          START EXPERIMENTING
        </a>
      </div>
    </div>
  </div>
</section>