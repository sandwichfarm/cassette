<script>
  import { terminalOutput, sendRequest, selectedCassettes, cassettes } from '../stores/cassette.js';
  import { onMount, afterUpdate } from 'svelte';
  
  let terminalContainer;
  let commandInput = '';
  let commandHistory = [];
  let historyIndex = -1;
  let autoScroll = true;
  
  // Scroll to bottom when new output is added
  afterUpdate(() => {
    if (autoScroll && terminalContainer) {
      terminalContainer.scrollTop = terminalContainer.scrollHeight;
    }
  });
  
  async function executeCommand() {
    if (!commandInput.trim()) return;
    
    // Add to history
    commandHistory = [...commandHistory, commandInput];
    historyIndex = commandHistory.length;
    
    // Parse command
    const cmd = commandInput.trim();
    commandInput = '';
    
    // Add command to output
    terminalOutput.update(output => [
      ...output,
      {
        type: 'command',
        text: `$ ${cmd}`,
        timestamp: new Date()
      }
    ]);
    
    // Handle commands
    if (cmd === 'clear' || cmd === 'cls') {
      terminalOutput.set([]);
    } else if (cmd === 'help') {
      showHelp();
    } else if (cmd.startsWith('req ')) {
      const request = cmd.substring(4);
      await sendRequest(request);
    } else if (cmd === 'list') {
      listCassettes();
    } else if (cmd === 'select all') {
      selectedCassettes.update(() => new Set(Array.from($cassettes.keys())));
      terminalOutput.update(output => [
        ...output,
        {
          type: 'system',
          text: '> All cassettes selected',
          timestamp: new Date()
        }
      ]);
    } else if (cmd === 'select none') {
      selectedCassettes.update(() => new Set());
      terminalOutput.update(output => [
        ...output,
        {
          type: 'system',
          text: '> All cassettes deselected',
          timestamp: new Date()
        }
      ]);
    } else {
      // Try to parse as JSON for direct requests
      try {
        const parsed = JSON.parse(cmd);
        if (Array.isArray(parsed) && parsed[0] === 'REQ') {
          await sendRequest(cmd);
        } else {
          terminalOutput.update(output => [
            ...output,
            {
              type: 'error',
              text: `> Unknown command: ${cmd}`,
              timestamp: new Date()
            }
          ]);
        }
      } catch {
        terminalOutput.update(output => [
          ...output,
          {
            type: 'error',
            text: `> Unknown command: ${cmd}`,
            timestamp: new Date()
          }
        ]);
      }
    }
  }
  
  function showHelp() {
    const helpText = `> AVAILABLE COMMANDS:
  clear/cls          - Clear terminal
  help              - Show this help
  list              - List loaded cassettes
  select all        - Select all cassettes
  select none       - Deselect all cassettes
  req <json>        - Send request to selected cassettes
  ["REQ",...]       - Direct JSON request format
  
> EXAMPLES:
  req ["REQ","sub1",{"kinds":[1],"limit":10}]
  ["REQ","test",{"authors":["..."]}]`;
    
    terminalOutput.update(output => [
      ...output,
      {
        type: 'system',
        text: helpText,
        timestamp: new Date()
      }
    ]);
  }
  
  function listCassettes() {
    if ($cassettes.size === 0) {
      terminalOutput.update(output => [
        ...output,
        {
          type: 'system',
          text: '> No cassettes loaded',
          timestamp: new Date()
        }
      ]);
      return;
    }
    
    let listText = '> LOADED CASSETTES:\n';
    for (const [id, cassette] of $cassettes) {
      const selected = $selectedCassettes.has(id) ? '[X]' : '[ ]';
      listText += `  ${selected} ${cassette.name} v${cassette.version} (${cassette.eventCount} events)\n`;
    }
    
    terminalOutput.update(output => [
      ...output,
      {
        type: 'system',
        text: listText.trim(),
        timestamp: new Date()
      }
    ]);
  }
  
  function handleKeyDown(e) {
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (historyIndex > 0) {
        historyIndex--;
        commandInput = commandHistory[historyIndex];
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (historyIndex < commandHistory.length - 1) {
        historyIndex++;
        commandInput = commandHistory[historyIndex];
      } else {
        historyIndex = commandHistory.length;
        commandInput = '';
      }
    }
  }
  
  function formatOutput(entry) {
    if (entry.type === 'response') {
      try {
        const parsed = JSON.parse(entry.text);
        return `[${entry.cassetteName}] ${JSON.stringify(parsed, null, 2)}`;
      } catch {
        return `[${entry.cassetteName}] ${entry.text}`;
      }
    }
    return entry.text;
  }
  
  onMount(() => {
    // Show welcome message
    terminalOutput.update(output => [
      ...output,
      {
        type: 'system',
        text: '> CASSETTE DECK TERMINAL v2.0\n> Type "help" for commands\n> Ready.',
        timestamp: new Date()
      }
    ]);
  });
</script>

<div class="terminal">
  <div class="terminal-header">
    <span class="terminal-title">TERMINAL</span>
    <label class="auto-scroll-toggle">
      <input type="checkbox" bind:checked={autoScroll} />
      AUTO-SCROLL
    </label>
  </div>
  
  <div class="terminal-output" bind:this={terminalContainer}>
    {#each $terminalOutput as entry}
      <div class="output-entry {entry.type}">
        <pre>{formatOutput(entry)}</pre>
      </div>
    {/each}
  </div>
  
  <div class="terminal-input">
    <span class="prompt">$</span>
    <input 
      type="text"
      bind:value={commandInput}
      on:keydown={handleKeyDown}
      on:keydown|preventDefault={(e) => e.key === 'Enter' && executeCommand()}
      placeholder="Enter command or JSON request..."
      autocomplete="off"
      spellcheck="false"
    />
  </div>
</div>

<style>
  .terminal {
    height: 100%;
    display: flex;
    flex-direction: column;
    font-family: 'Share Tech Mono', monospace;
  }
  
  .terminal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px;
    border-bottom: 1px solid #0f0;
    background: rgba(0, 255, 0, 0.1);
  }
  
  .terminal-title {
    font-size: 14px;
    color: #0f0;
    letter-spacing: 0.1em;
  }
  
  .auto-scroll-toggle {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: #0a0;
  }
  
  .auto-scroll-toggle input {
    cursor: pointer;
  }
  
  .terminal-output {
    flex: 1;
    overflow-y: auto;
    padding: 10px;
    background: #000;
  }
  
  .output-entry {
    margin-bottom: 10px;
    white-space: pre-wrap;
    word-break: break-all;
  }
  
  .output-entry pre {
    margin: 0;
    font-family: inherit;
  }
  
  .output-entry.command {
    color: #0f0;
  }
  
  .output-entry.system {
    color: #0a0;
  }
  
  .output-entry.error {
    color: #f00;
  }
  
  .output-entry.request {
    color: #0ff;
  }
  
  .output-entry.response {
    color: #ff0;
  }
  
  .terminal-input {
    display: flex;
    align-items: center;
    padding: 10px;
    border-top: 1px solid #0f0;
    background: rgba(0, 255, 0, 0.05);
  }
  
  .prompt {
    color: #0f0;
    margin-right: 10px;
    font-weight: bold;
  }
  
  .terminal-input input {
    flex: 1;
    background: transparent;
    border: none;
    color: #0f0;
    font-family: 'Share Tech Mono', monospace;
    font-size: 14px;
    outline: none;
  }
  
  .terminal-input input::placeholder {
    color: #0a0;
    opacity: 0.5;
  }
  
  /* Custom scrollbar */
  .terminal-output::-webkit-scrollbar {
    width: 8px;
  }
  
  .terminal-output::-webkit-scrollbar-track {
    background: rgba(0, 255, 0, 0.1);
  }
  
  .terminal-output::-webkit-scrollbar-thumb {
    background: #0a0;
    border-radius: 4px;
  }
  
  .terminal-output::-webkit-scrollbar-thumb:hover {
    background: #0f0;
  }
</style>