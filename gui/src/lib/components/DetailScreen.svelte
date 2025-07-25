<script>
  export let item = null;
  
  function formatTimestamp(timestamp) {
    return new Date(timestamp * 1000).toLocaleString();
  }
  
  function formatPubkey(pubkey) {
    if (!pubkey) return '';
    return pubkey.substring(0, 16) + '...' + pubkey.substring(pubkey.length - 16);
  }
</script>

<div class="detail-screen">
  <div class="screen-frame">
    <div class="screen-label">DETAIL VIEW</div>
    
    <div class="screen-content">
      {#if item}
        <div class="detail-grid">
          <!-- Common fields -->
          <div class="detail-row">
            <span class="detail-label">ID:</span>
            <span class="detail-value">{formatPubkey(item.id)}</span>
          </div>
          
          <div class="detail-row">
            <span class="detail-label">PUBKEY:</span>
            <span class="detail-value">{formatPubkey(item.pubkey)}</span>
          </div>
          
          <div class="detail-row">
            <span class="detail-label">TIME:</span>
            <span class="detail-value">{formatTimestamp(item.created_at)}</span>
          </div>
          
          <!-- Kind-specific details -->
          {#if item.kind === 0}
            <!-- Profile details -->
            {#if item.picture}
              <div class="detail-row full-width">
                <img src={item.picture} alt={item.name} class="profile-image" />
              </div>
            {/if}
            
            <div class="detail-row">
              <span class="detail-label">NAME:</span>
              <span class="detail-value">{item.name}</span>
            </div>
            
            {#if item.about}
              <div class="detail-row full-width">
                <span class="detail-label">ABOUT:</span>
                <div class="detail-text">{item.about}</div>
              </div>
            {/if}
            
          {:else if item.kind === 10002}
            <!-- Relay list details -->
            <div class="detail-row">
              <span class="detail-label">RELAYS:</span>
              <span class="detail-value">{item.relayCount}</span>
            </div>
            
            <div class="detail-row full-width">
              <span class="detail-label">RELAY URLS:</span>
              <div class="tag-list">
                {#each item.tags.filter(t => t[0] === 'r') as relay}
                  <div class="relay-url">{relay[1]}</div>
                {/each}
              </div>
            </div>
            
          {:else if item.kind === 1}
            <!-- Note details -->
            <div class="detail-row full-width">
              <span class="detail-label">CONTENT:</span>
              <div class="detail-text">{item.content}</div>
            </div>
            
            {#if item.topics && item.topics.length > 0}
              <div class="detail-row">
                <span class="detail-label">TOPICS:</span>
                <div class="topic-list">
                  {#each item.topics as topic}
                    <span class="topic-tag">#{topic}</span>
                  {/each}
                </div>
              </div>
            {/if}
            
          {:else if item.kind === 7}
            <!-- Reaction details -->
            <div class="detail-row">
              <span class="detail-label">REACTION:</span>
              <span class="detail-value reaction">{item.reaction}</span>
            </div>
            
            {#if item.targetEvent}
              <div class="detail-row">
                <span class="detail-label">TARGET:</span>
                <span class="detail-value">{formatPubkey(item.targetEvent)}</span>
              </div>
            {/if}
          {/if}
          
          <!-- Raw tags -->
          {#if item.tags && item.tags.length > 0}
            <div class="detail-row full-width">
              <span class="detail-label">TAGS:</span>
              <div class="tag-list">
                {#each item.tags as tag}
                  <div class="tag-item">[{tag.join(', ')}]</div>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {:else}
        <div class="no-selection">
          <div class="placeholder-icon">📊</div>
          <div class="placeholder-text">SELECT AN ITEM TO VIEW DETAILS</div>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .detail-screen {
    background: #2a2a2a;
    padding: 20px;
    border: 1px solid #333;
    min-height: 400px;
  }
  
  .screen-frame {
    background: #1a1a1a;
    border: 2px solid #444;
    border-radius: 4px;
    height: 100%;
    display: flex;
    flex-direction: column;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.5);
  }
  
  .screen-label {
    background: linear-gradient(to bottom, #3a3a3a, #2a2a2a);
    padding: 10px;
    text-align: center;
    font-size: 12px;
    color: #888;
    letter-spacing: 0.2em;
    border-bottom: 1px solid #222;
  }
  
  .screen-content {
    flex: 1;
    padding: 20px;
    background: #000;
    color: #0f0;
    font-family: 'Share Tech Mono', monospace;
    overflow-y: auto;
  }
  
  .detail-grid {
    display: grid;
    gap: 10px;
  }
  
  .detail-row {
    display: grid;
    grid-template-columns: 100px 1fr;
    gap: 10px;
    align-items: start;
  }
  
  .detail-row.full-width {
    grid-template-columns: 1fr;
  }
  
  .detail-label {
    font-size: 11px;
    color: #0a0;
    text-align: right;
  }
  
  .detail-value {
    font-size: 11px;
    color: #0f0;
    word-break: break-all;
  }
  
  .detail-value.reaction {
    font-size: 24px;
  }
  
  .detail-text {
    font-size: 11px;
    color: #0f0;
    margin-top: 5px;
    white-space: pre-wrap;
    word-break: break-word;
  }
  
  .profile-image {
    width: 120px;
    height: 120px;
    border-radius: 8px;
    object-fit: cover;
    margin: 10px auto;
    display: block;
    border: 2px solid #0f0;
  }
  
  .tag-list, .topic-list {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin-top: 5px;
  }
  
  .tag-item, .relay-url {
    font-size: 10px;
    color: #0a0;
    background: rgba(0, 255, 0, 0.1);
    padding: 4px 6px;
    border-radius: 2px;
    word-break: break-all;
  }
  
  .topic-tag {
    font-size: 10px;
    color: #0f0;
    background: rgba(0, 255, 0, 0.2);
    padding: 2px 6px;
    border-radius: 2px;
    display: inline-block;
    margin-right: 5px;
  }
  
  .no-selection {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 300px;
    gap: 20px;
  }
  
  .placeholder-icon {
    font-size: 48px;
    opacity: 0.3;
  }
  
  .placeholder-text {
    font-size: 12px;
    color: #0a0;
    opacity: 0.5;
    letter-spacing: 0.1em;
  }
  
  /* Scrollbar */
  .screen-content::-webkit-scrollbar {
    width: 8px;
  }
  
  .screen-content::-webkit-scrollbar-track {
    background: rgba(0, 255, 0, 0.1);
  }
  
  .screen-content::-webkit-scrollbar-thumb {
    background: #0a0;
    border-radius: 4px;
  }
  
  .screen-content::-webkit-scrollbar-thumb:hover {
    background: #0f0;
  }
</style>