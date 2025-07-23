<script>
  import { createEventDispatcher, onMount } from 'svelte';
  import { cassettes, sendRequest, terminalOutput } from '../stores/cassette.js';
  
  export let preset = null;
  export let currentPage = 0;
  export let itemsPerPage = 9;
  
  const dispatch = createEventDispatcher();
  
  let items = [];
  let loading = false;
  let totalPages = 0;
  
  $: if (preset && $cassettes.size > 0) {
    loadPresetData();
  }
  
  async function loadPresetData() {
    if (!preset || $cassettes.size === 0) return;
    
    loading = true;
    items = [];
    
    // Create filter based on preset
    const filter = { kinds: [preset.kind] };
    if (preset.tags) {
      filter.tags = preset.tags;
    }
    
    // Add pagination
    filter.limit = itemsPerPage;
    filter.offset = currentPage * itemsPerPage;
    
    const request = JSON.stringify(['REQ', `preset-${preset.id}-${Date.now()}`, filter]);
    
    try {
      // Get first cassette
      const firstCassette = Array.from($cassettes.values())[0];
      if (!firstCassette) return;
      
      const response = firstCassette.cassette.methods.req(request);
      const events = JSON.parse(response);
      
      // Process events into display items
      items = events.slice(0, itemsPerPage).map(event => {
        const item = {
          id: event.id,
          kind: event.kind,
          created_at: event.created_at,
          pubkey: event.pubkey,
          content: event.content,
          tags: event.tags
        };
        
        // Extract specific data based on kind
        if (preset.kind === 0) {
          // Profile metadata
          try {
            const metadata = JSON.parse(event.content);
            item.name = metadata.name || metadata.display_name || 'Anonymous';
            item.picture = metadata.picture;
            item.about = metadata.about;
          } catch (e) {
            item.name = 'Invalid Profile';
          }
        } else if (preset.kind === 10002) {
          // Relay list
          item.relayCount = event.tags.filter(t => t[0] === 'r').length;
          item.name = `${item.relayCount} relays`;
        } else if (preset.kind === 1) {
          // Note/Reply
          item.summary = event.content.substring(0, 100);
          const topicTags = event.tags.filter(t => t[0] === 't');
          item.topics = topicTags.map(t => t[1]);
        } else if (preset.kind === 7) {
          // Reaction
          item.reaction = event.content;
          const eTags = event.tags.filter(t => t[0] === 'e');
          item.targetEvent = eTags[0]?.[1];
        }
        
        return item;
      });
      
      // Estimate total pages (cassette might not return total count)
      totalPages = events.length === itemsPerPage ? 10 : currentPage + 1;
      
    } catch (error) {
      console.error('Failed to load preset data:', error);
      terminalOutput.update(output => [...output, {
        type: 'error',
        text: `Failed to load ${preset.label}: ${error.message}`,
        timestamp: new Date()
      }]);
    } finally {
      loading = false;
    }
  }
  
  function formatPubkey(pubkey) {
    return pubkey.substring(0, 8) + '...';
  }
</script>

<div class="data-display">
  <div class="display-header">
    <div class="header-info">
      {#if preset}
        <span class="preset-name">{preset.label}</span>
        <span class="item-count">{items.length} items</span>
      {:else}
        <span class="no-preset">SELECT A PRESET</span>
      {/if}
    </div>
    
    <div class="pagination">
      <button 
        class="page-button"
        disabled={currentPage === 0}
        on:click={() => dispatch('prevPage')}
      >
        <span class="button-icon">◀</span>
      </button>
      
      <span class="page-info">PAGE {currentPage + 1}</span>
      
      <button 
        class="page-button"
        disabled={currentPage >= totalPages - 1}
        on:click={() => dispatch('nextPage')}
      >
        <span class="button-icon">▶</span>
      </button>
    </div>
  </div>
  
  <div class="data-grid">
    {#if loading}
      <div class="loading">LOADING...</div>
    {:else if items.length > 0}
      {#each items as item, i}
        <button 
          class="data-button"
          on:click={() => dispatch('select', item)}
        >
          {#if preset?.kind === 0}
            <!-- Profile display -->
            {#if item.picture}
              <img src={item.picture} alt={item.name} class="profile-pic" />
            {:else}
              <div class="profile-placeholder">
                {item.name.charAt(0).toUpperCase()}
              </div>
            {/if}
            <div class="item-label">{item.name}</div>
          {:else if preset?.kind === 10002}
            <!-- Relay list display -->
            <div class="relay-icon">📡</div>
            <div class="item-label">{item.relayCount} RELAYS</div>
            <div class="item-sublabel">{formatPubkey(item.pubkey)}</div>
          {:else if preset?.kind === 1}
            <!-- Note display -->
            <div class="note-preview">{item.summary}</div>
            {#if item.topics.length > 0}
              <div class="topics">
                {#each item.topics.slice(0, 2) as topic}
                  <span class="topic">#{topic}</span>
                {/each}
              </div>
            {/if}
          {:else if preset?.kind === 7}
            <!-- Reaction display -->
            <div class="reaction-emoji">{item.reaction}</div>
            <div class="item-sublabel">→ {formatPubkey(item.targetEvent || '')}</div>
          {/if}
        </button>
      {/each}
      
      <!-- Fill empty slots -->
      {#each Array(itemsPerPage - items.length) as _}
        <div class="data-button empty">
          <span class="empty-label">EMPTY</span>
        </div>
      {/each}
    {:else if preset}
      <!-- All empty slots -->
      {#each Array(itemsPerPage) as _}
        <div class="data-button empty">
          <span class="empty-label">EMPTY</span>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .data-display {
    background: #2a2a2a;
    padding: 20px;
    border: 1px solid #333;
  }
  
  .display-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
    padding-bottom: 15px;
    border-bottom: 1px solid #444;
  }
  
  .header-info {
    display: flex;
    align-items: center;
    gap: 15px;
  }
  
  .preset-name {
    font-size: 14px;
    color: #0f0;
    letter-spacing: 0.1em;
    text-shadow: 0 0 5px #0f0;
  }
  
  .item-count {
    font-size: 12px;
    color: #888;
  }
  
  .no-preset {
    font-size: 12px;
    color: #666;
    letter-spacing: 0.1em;
  }
  
  .pagination {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  
  .page-button {
    width: 30px;
    height: 30px;
    background: linear-gradient(to bottom, #3a3a3a, #2a2a2a);
    border: 2px solid #444;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.1s;
    box-shadow: 
      0 2px 4px rgba(0, 0, 0, 0.3),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
  }
  
  .page-button:hover:not(:disabled) {
    background: linear-gradient(to bottom, #4a4a4a, #3a3a3a);
  }
  
  .page-button:active:not(:disabled) {
    transform: translateY(1px);
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.5);
  }
  
  .page-button:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
  
  .button-icon {
    color: #0f0;
    font-size: 12px;
  }
  
  .page-info {
    font-size: 11px;
    color: #888;
    letter-spacing: 0.1em;
    min-width: 60px;
    text-align: center;
  }
  
  .data-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }
  
  .data-button {
    aspect-ratio: 1;
    background: linear-gradient(to bottom, #3a3a3a, #2a2a2a);
    border: 2px solid #444;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 5px;
    padding: 10px;
    transition: all 0.1s;
    position: relative;
    overflow: hidden;
    box-shadow: 
      0 2px 4px rgba(0, 0, 0, 0.3),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
  }
  
  .data-button:hover {
    background: linear-gradient(to bottom, #4a4a4a, #3a3a3a);
    border-color: #0a0;
  }
  
  .data-button:active {
    transform: scale(0.98);
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.5);
  }
  
  .data-button.empty {
    opacity: 0.3;
    cursor: default;
  }
  
  .loading {
    grid-column: 1 / -1;
    text-align: center;
    color: #0f0;
    padding: 40px;
    font-size: 14px;
    letter-spacing: 0.2em;
  }
  
  /* Profile styles */
  .profile-pic {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    object-fit: cover;
  }
  
  .profile-placeholder {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    background: #0f0;
    color: #000;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 20px;
    font-weight: bold;
  }
  
  /* Item labels */
  .item-label {
    font-size: 10px;
    color: #ccc;
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 100%;
  }
  
  .item-sublabel {
    font-size: 9px;
    color: #666;
    text-align: center;
  }
  
  /* Note styles */
  .note-preview {
    font-size: 9px;
    color: #aaa;
    text-align: left;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    width: 100%;
  }
  
  .topics {
    display: flex;
    gap: 5px;
    flex-wrap: wrap;
  }
  
  .topic {
    font-size: 8px;
    color: #0a0;
    background: rgba(0, 255, 0, 0.1);
    padding: 2px 4px;
    border-radius: 2px;
  }
  
  /* Relay styles */
  .relay-icon {
    font-size: 24px;
  }
  
  /* Reaction styles */
  .reaction-emoji {
    font-size: 32px;
  }
  
  .empty-label {
    font-size: 10px;
    color: #555;
    letter-spacing: 0.1em;
  }
  
  @media (max-width: 768px) {
    .data-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>