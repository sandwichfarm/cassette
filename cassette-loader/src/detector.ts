/**
 * Utility to detect WASM module target environment 
 */

/**
 * Analyzes WebAssembly exports to detect if it was compiled for Node.js or browser environments
 * @param exports The WebAssembly exports to analyze
 * @returns Object containing detection results
 */
export function detectWasmCompilationTarget(exports: WebAssembly.Exports): {
  isNodeTargeted: boolean;
  isBrowserTargeted: boolean;
  isRustCompiled: boolean;
  isEmscriptenCompiled: boolean;
  usesNodeAPIs: boolean;
  details: string[];
} {
  const exportNames = Object.keys(exports);
  const details: string[] = [];
  
  // Pattern detection
  let isNodeTargeted = false;
  let isBrowserTargeted = false;
  let isRustCompiled = false;
  let isEmscriptenCompiled = false;
  let usesNodeAPIs = false;
  
  // Check Rust patterns
  if (exportNames.includes('memory') && 
      (exportNames.includes('__wbindgen_malloc') || exportNames.includes('__wbindgen_free'))) {
    isRustCompiled = true;
    details.push('Detected Rust compilation patterns (wbindgen)');
  }
  
  // Check Emscripten patterns
  if (exportNames.includes('stackSave') && 
      exportNames.includes('stackRestore') && 
      exportNames.includes('stackAlloc')) {
    isEmscriptenCompiled = true;
    details.push('Detected Emscripten compilation patterns');
  }
  
  // Check for Node.js specific exports
  if (exportNames.includes('__wbg_process_e56fd54cf6319b6c') ||
      exportNames.includes('__wbg_versions_77e21455908dad33') ||
      exportNames.includes('__wbindgen_is_node') ||
      exportNames.some(name => name.includes('node_modules'))) {
    isNodeTargeted = true;
    usesNodeAPIs = true;
    details.push('⚠️ Detected Node.js specific exports - might require Node.js environment');
  }
  
  // Check for browser specific exports
  if (exportNames.includes('__wbg_window_f2557cc78490aceb') ||
      exportNames.includes('__wbg_document_1c64944725c0d81d') ||
      exportNames.includes('__wbg_navigator_480e592af6ad968b') ||
      exportNames.includes('__wbindgen_is_browser') ||
      exportNames.some(name => name.includes('Window')) ||
      exportNames.some(name => name.includes('Document'))) {
    isBrowserTargeted = true;
    details.push('Detected browser specific exports');
  }
  
  // Check for Web-specific APIs
  if (exportNames.includes('__wbg_fetch_0fe04905cccfc2aa') ||
      exportNames.includes('__wbg_crypto_2bc4d5b05161de5b') ||
      exportNames.some(name => name.includes('fetch')) ||
      exportNames.some(name => name.includes('Fetch')) ||
      exportNames.some(name => name.includes('XHR'))) {
    isBrowserTargeted = true;
    details.push('Detected Web API specific exports');
  }
  
  // Check for special string helpers
  if (exportNames.includes('__wbindgen_string_new') || 
      exportNames.includes('__wbindgen_string_get')) {
    details.push('Detected wbindgen string helpers that convert between JS and Rust strings');
  }
  
  // Check for most common NIP-01 specific exports
  if (exportNames.includes('req') || exportNames.includes('process')) {
    details.push('✅ Detected NIP-01 interface exports (req/process)');
  }
  
  // Extra diagnostic information
  if (exportNames.includes('memory')) {
    details.push('Module exports WebAssembly memory (standard approach)');
  }
  
  // Determine overall targeting
  if (!isNodeTargeted && !isBrowserTargeted) {
    details.push('No environment-specific exports detected, should work in any environment');
    // Default to both since it's likely platform-agnostic
    isBrowserTargeted = true; 
    isNodeTargeted = true;
  }
  
  return {
    isNodeTargeted,
    isBrowserTargeted,
    isRustCompiled,
    isEmscriptenCompiled,
    usesNodeAPIs,
    details
  };
} 