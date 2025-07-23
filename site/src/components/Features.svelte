<script>
  import { onMount } from 'svelte'
  
  let featuresElement
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
    
    if (featuresElement) observer.observe(featuresElement)
    
    return () => {
      if (featuresElement) observer.unobserve(featuresElement)
    }
  })
  
  const features = [
    {
      icon: '📦',
      title: 'Portable Storage',
      description: 'Pack Nostr events into WebAssembly modules that run anywhere - browsers, servers, edge workers, or CLI.'
    },
    {
      icon: '🔍',
      title: 'NIP-01 Compatible',
      description: 'Query cassettes using standard Nostr relay protocol. Works seamlessly with existing Nostr tools.'
    },
    {
      icon: '⚡',
      title: 'Lightning Fast',
      description: 'WebAssembly performance means instant queries. No network latency, no database overhead.'
    },
    {
      icon: '🔒',
      title: 'Privacy First',
      description: 'Keep events local while maintaining relay compatibility. Perfect for sensitive or personal data.'
    },
    {
      icon: '🎯',
      title: 'Deterministic Testing',
      description: 'Create reproducible test fixtures for Nostr clients. Same events, same results, every time.'
    },
    {
      icon: '🌐',
      title: 'Offline Ready',
      description: 'Query events without network access. Perfect for mobile apps and unreliable connections.'
    }
  ]
</script>

<section bind:this={featuresElement} class="py-20 bg-gray-50">
  <div class="container mx-auto px-6">
    <div class={`text-center mb-16 transition-all duration-700 ${visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-10'}`}>
      <h2 class="text-4xl md:text-5xl font-bold mb-4 text-gray-900">
        Why Cassette?
      </h2>
      <p class="text-xl text-gray-600 max-w-2xl mx-auto">
        Transform how you store, share, and query Nostr events with the power of WebAssembly
      </p>
    </div>
    
    <div class="grid md:grid-cols-2 lg:grid-cols-3 gap-8">
      {#each features as feature, i}
        <div 
          class={`bg-white rounded-xl p-8 shadow-lg hover:shadow-xl transition-all duration-300 transform hover:-translate-y-1 ${visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-10'}`}
          style="transition-delay: {i * 100}ms"
        >
          <div class="text-4xl mb-4">{feature.icon}</div>
          <h3 class="text-2xl font-bold mb-3 text-gray-900">{feature.title}</h3>
          <p class="text-gray-600">{feature.description}</p>
        </div>
      {/each}
    </div>
  </div>
</section>