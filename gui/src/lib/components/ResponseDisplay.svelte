<script>
  import { cassetteState } from '../stores/cassette.js';
  
  let autoScroll = true;
  let container;
  
  $: if (autoScroll && container && $cassetteState.responses.length > 0) {
    setTimeout(() => {
      container.scrollTop = container.scrollHeight;
    }, 50);
  }
  
  function formatResponse(response) {
    try {
      const parsed = JSON.parse(response);
      return JSON.stringify(parsed, null, 2);
    } catch {
      return response;
    }
  }
  
  function getResponseType(response) {
    try {
      const parsed = JSON.parse(response);
      if (Array.isArray(parsed)) {
        return parsed[0];
      }
      return 'UNKNOWN';
    } catch {
      return 'ERROR';
    }
  }
</script>

<div class="response-display">
  <div class="header">
    <h3>RESPONSES</h3>
    <label class="auto-scroll">
      <input type="checkbox" bind:checked={autoScroll} />
      Auto-scroll
    </label>
  </div>
  
  <div class="response-container" bind:this={container}>
    {#if $cassetteState.responses.length === 0}
      <div class="empty-state">
        No responses yet. Load a cassette and press PLAY to send requests.
      </div>
    {:else}
      {#each $cassetteState.responses as response, i}
        <div class="response-item">
          <div class="response-header">
            <span class="response-type type-{getResponseType(response).toLowerCase()}">
              {getResponseType(response)}
            </span>
            <span class="response-time">#{i + 1}</span>
          </div>
          <pre class="response-content">{formatResponse(response)}</pre>
        </div>
      {/each}
    {/if}
  </div>
  
  {#if $cassetteState.responses.length > 0}
    <div class="footer">
      <button 
        class="clear-button"
        on:click={() => cassetteState.update(s => ({ ...s, responses: [] }))}
      >
        Clear All
      </button>
      <span class="response-count">{$cassetteState.responses.length} responses</span>
    </div>
  {/if}
</div>

<style>
  .response-display {
    background: #1a1a1a;
    border: 2px solid #333;
    border-radius: 8px;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .header {
    padding: 12px 16px;
    border-bottom: 1px solid #333;
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: #222;
  }

  .header h3 {
    margin: 0;
    color: #c0c0c0;
    font-size: 14px;
    letter-spacing: 1px;
  }

  .auto-scroll {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #888;
    font-size: 12px;
  }

  .auto-scroll input {
    cursor: pointer;
  }

  .response-container {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }

  .empty-state {
    color: #666;
    text-align: center;
    padding: 40px 20px;
    font-style: italic;
  }

  .response-item {
    margin-bottom: 16px;
    background: #0a0a0a;
    border: 1px solid #333;
    border-radius: 4px;
    overflow: hidden;
  }

  .response-header {
    padding: 8px 12px;
    background: #222;
    border-bottom: 1px solid #333;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .response-type {
    font-size: 12px;
    font-weight: bold;
    padding: 2px 8px;
    border-radius: 3px;
    text-transform: uppercase;
  }

  .type-event {
    background: #2a5c39;
    color: #7fbf7f;
  }

  .type-notice {
    background: #5c4a2a;
    color: #ffbf7f;
  }

  .type-eose {
    background: #2a4a5c;
    color: #7fbfff;
  }

  .type-error {
    background: #5c2a2a;
    color: #ff7f7f;
  }

  .type-unknown {
    background: #3a3a3a;
    color: #999;
  }

  .response-time {
    color: #666;
    font-size: 12px;
  }

  .response-content {
    padding: 12px;
    margin: 0;
    color: #c0c0c0;
    font-family: monospace;
    font-size: 12px;
    line-height: 1.4;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .footer {
    padding: 12px 16px;
    border-top: 1px solid #333;
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: #222;
  }

  .clear-button {
    background: #333;
    border: 1px solid #555;
    border-radius: 4px;
    padding: 4px 12px;
    color: #999;
    font-size: 12px;
    cursor: pointer;
    transition: all 0.1s ease;
  }

  .clear-button:hover {
    background: #444;
    border-color: #666;
    color: #bbb;
  }

  .response-count {
    color: #666;
    font-size: 12px;
  }

  /* Custom scrollbar */
  .response-container::-webkit-scrollbar {
    width: 8px;
  }

  .response-container::-webkit-scrollbar-track {
    background: #0a0a0a;
  }

  .response-container::-webkit-scrollbar-thumb {
    background: #333;
    border-radius: 4px;
  }

  .response-container::-webkit-scrollbar-thumb:hover {
    background: #444;
  }
</style>