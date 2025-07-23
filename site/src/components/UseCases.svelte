<script>
  import { onMount } from 'svelte'
  
  let usecasesElement
  let visible = false
  
  onMount(() => {
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          visible = true
        }
      },
      { threshold: 0.1 }
    )
    
    if (usecasesElement) observer.observe(usecasesElement)
    
    return () => {
      if (usecasesElement) observer.unobserve(usecasesElement)
    }
  })
  
  const useCases = [
    {
      title: 'Event Archival',
      description: 'Preserve important Nostr events in a portable, immutable format. Perfect for backing up your social graph, notes, or relay data.',
      icon: '💾'
    },
    {
      title: 'Offline Applications',
      description: 'Build Nostr clients that work without internet. Query events locally with full NIP-01 compatibility.',
      icon: '📱'
    },
    {
      title: 'Test Fixtures',
      description: 'Create deterministic test data for Nostr applications. Same events, same behavior, every test run.',
      icon: '🧪'
    },
    {
      title: 'Content Distribution',
      description: 'Share curated event collections as single files. Perfect for distributing reading lists, conversations, or topic compilations.',
      icon: '📚'
    },
    {
      title: 'Edge Computing',
      description: 'Deploy Nostr relays to edge workers using WebAssembly. Serve events closer to users with minimal infrastructure.',
      icon: '🌍'
    },
    {
      title: 'Privacy Protection',
      description: 'Keep sensitive events local while maintaining standard query interfaces. No external relay needed.',
      icon: '🔐'
    }
  ]
</script>

<section bind:this={usecasesElement} class="py-20 bg-gray-50">
  <div class="container mx-auto px-6">
    <div class={`text-center mb-16 transition-all duration-700 ${visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-10'}`}>
      <h2 class="text-4xl md:text-5xl font-bold mb-4 text-gray-900">
        Use Cases
      </h2>
      <p class="text-xl text-gray-600 max-w-2xl mx-auto">
        Discover how teams are using Cassette to revolutionize their Nostr workflows
      </p>
    </div>
    
    <div class="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
      {#each useCases as useCase, i}
        <div 
          class={`group bg-white rounded-xl p-8 shadow-md hover:shadow-xl transition-all duration-300 border border-gray-100 hover:border-cassette-purple/20 ${visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-10'}`}
          style="transition-delay: {i * 100}ms"
        >
          <div class="text-4xl mb-4 group-hover:scale-110 transition-transform duration-300">{useCase.icon}</div>
          <h3 class="text-xl font-bold mb-3 text-gray-900 group-hover:text-cassette-purple transition-colors duration-300">
            {useCase.title}
          </h3>
          <p class="text-gray-600">
            {useCase.description}
          </p>
        </div>
      {/each}
    </div>
    
    <div class={`mt-16 text-center transition-all duration-700 delay-500 ${visible ? 'opacity-100' : 'opacity-0'}`}>
      <div class="bg-gradient-to-r from-cassette-purple to-cassette-blue p-12 rounded-2xl text-white">
        <h3 class="text-3xl font-bold mb-4">Ready to Get Started?</h3>
        <p class="text-xl mb-8 opacity-90">Join the growing community of developers using Cassette</p>
        <div class="flex flex-col sm:flex-row gap-4 justify-center">
          <a 
            href="https://github.com/dskvr/cassette/releases/latest" 
            target="_blank" 
            rel="noopener noreferrer"
            class="px-8 py-4 bg-white text-cassette-purple rounded-lg font-semibold hover:shadow-lg transform hover:scale-105 transition-all duration-200"
          >
            Download Latest Release
          </a>
          <a 
            href="https://github.com/dskvr/cassette/discussions" 
            target="_blank" 
            rel="noopener noreferrer"
            class="px-8 py-4 bg-transparent border-2 border-white rounded-lg font-semibold hover:bg-white hover:text-cassette-purple transition-all duration-200"
          >
            Join Discussion
          </a>
        </div>
      </div>
    </div>
  </div>
</section>