<script>
  import { onMount } from 'svelte'
  
  let quickstartElement
  let visible = false
  let activeTab = 'create'
  
  onMount(() => {
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          visible = true
        }
      },
      { threshold: 0.1 }
    )
    
    if (quickstartElement) observer.observe(quickstartElement)
    
    return () => {
      if (quickstartElement) observer.unobserve(quickstartElement)
    }
  })
  
  const codeExamples = {
    create: {
      title: 'Create a Cassette',
      code: `# From a relay
nak req -k 1 -l 100 wss://nos.lol | cassette record --name my-notes

# From a file
cassette record events.json --name my-notes

# Output: my-notes.wasm`
    },
    query: {
      title: 'Query Events',
      code: `# Get all events
cassette req my-notes.wasm

# Filter by kind
cassette req my-notes.wasm --kinds 1

# Filter by author
cassette req my-notes.wasm --authors npub1...

# Multiple filters
cassette req my-notes.wasm --kinds 1 --kinds 7 --limit 10`
    },
    combine: {
      title: 'Combine Cassettes',
      code: `# Merge cassettes
cassette dub alice.wasm bob.wasm combined.wasm

# Merge with filters
cassette dub *.wasm filtered.wasm --kinds 1 --since 1700000000`
    }
  }
</script>

<section id="quickstart" bind:this={quickstartElement} class="py-20 bg-gradient-to-b from-gray-900 to-cassette-dark text-white">
  <div class="container mx-auto px-6">
    <div class={`text-center mb-16 transition-all duration-700 ${visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-10'}`}>
      <h2 class="text-4xl md:text-5xl font-bold mb-4">
        Quick Start
      </h2>
      <p class="text-xl text-gray-300 max-w-2xl mx-auto">
        Get up and running with Cassette in minutes
      </p>
    </div>
    
    <div class={`max-w-4xl mx-auto transition-all duration-700 delay-200 ${visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-10'}`}>
      <div class="mb-8">
        <div class="bg-gray-800 rounded-lg p-6 mb-8">
          <h3 class="text-2xl font-bold mb-4 flex items-center gap-2">
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M9 19l3 3m0 0l3-3m-3 3V10"></path>
            </svg>
            Install
          </h3>
          <div class="bg-gray-900 rounded p-4 font-mono text-sm">
            <p class="text-gray-400 mb-2"># Download the latest binary</p>
            <p class="text-green-400">wget https://github.com/dskvr/cassette/releases/latest/download/cassette-linux-amd64</p>
            <p class="text-green-400">chmod +x cassette-linux-amd64</p>
            <p class="text-green-400">sudo mv cassette-linux-amd64 /usr/local/bin/cassette</p>
          </div>
        </div>
        
        <div class="flex gap-2 mb-6">
          {#each Object.keys(codeExamples) as tab}
            <button
              on:click={() => activeTab = tab}
              class={`px-4 py-2 rounded-lg font-semibold transition-all duration-200 ${
                activeTab === tab 
                  ? 'bg-cassette-purple text-white' 
                  : 'bg-gray-800 text-gray-400 hover:text-white'
              }`}
            >
              {codeExamples[tab].title}
            </button>
          {/each}
        </div>
        
        <div class="bg-gray-800 rounded-lg p-6">
          <div class="bg-gray-900 rounded p-4 font-mono text-sm overflow-x-auto">
            <pre class="text-green-400">{codeExamples[activeTab].code}</pre>
          </div>
        </div>
      </div>
      
      <div class="text-center">
        <a 
          href="https://github.com/dskvr/cassette#readme" 
          target="_blank" 
          rel="noopener noreferrer"
          class="inline-flex items-center gap-2 px-6 py-3 bg-cassette-purple hover:bg-purple-700 rounded-lg font-semibold transition-all duration-200"
        >
          View Full Documentation
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3"></path>
          </svg>
        </a>
      </div>
    </div>
  </div>
</section>