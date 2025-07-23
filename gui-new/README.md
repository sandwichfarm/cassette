# Cassette Player GUI

A retro-styled single-page application for loading and testing Nostr event cassettes (WASM modules) with an analog cassette player aesthetic.

## Features

- 🎵 **Analog cassette player UI** with spinning reels and VU meters
- 📼 **Drag-and-drop WASM loading** from filesystem
- 🎛️ **Interactive controls** - Play, Stop, Rewind, Eject
- 📊 **Real-time response display** with syntax highlighting
- 🎨 **Retro aesthetic** with attention to detail

## Setup

```bash
# Install dependencies
npm install

# Start development server
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

## Usage

1. **Load a Cassette**: Click the "LOAD" button and select a `.wasm` file
2. **View Metadata**: The cassette label shows event count, version, and description
3. **Send Requests**: Press PLAY to open the request editor
4. **View Responses**: Watch responses stream in real-time in the output display
5. **Control Playback**: Use Stop, Rewind, and Eject buttons

## Architecture

Built with:
- **Svelte** - Reactive UI framework
- **Vite** - Fast build tool
- **Tailwind CSS** - Utility-first styling
- **cassette-loader** - WASM loading and NIP-01 interface

### Project Structure

```
gui-new/
├── src/
│   ├── lib/
│   │   ├── components/     # UI components
│   │   ├── stores/         # Svelte stores for state
│   │   └── utils/          # Cassette loading utilities
│   ├── App.svelte         # Main app component
│   ├── main.js            # Entry point
│   └── app.css            # Global styles
├── public/                # Static assets
└── index.html             # HTML template
```

## Components

- **CassettePlayer** - Main player interface with tape mechanism
- **CassetteSlot** - File loading and metadata display
- **TapeReel** - Animated spinning tape reels
- **VUMeter** - Audio-style level meters
- **ControlButtons** - Play, Stop, Rewind, Eject controls
- **ResponseDisplay** - NIP-01 message viewer

## Customization

### Styling

The design uses custom Tailwind colors defined in `tailwind.config.js`:
- `cassette-black`: #1a1a1a
- `cassette-silver`: #c0c0c0
- `cassette-red`: #ff3333
- `tape-brown`: #8b4513

### Request Templates

Edit the default request in `ControlButtons.svelte`:
```javascript
let requestText = JSON.stringify(["REQ", "sub1", { "kinds": [1], "limit": 10 }], null, 2);
```

## Development

```bash
# Run with hot reload
npm run dev

# Type checking (if TypeScript is added)
npm run check

# Linting
npm run lint

# Format code
npm run format
```

## Browser Support

- Chrome/Edge 90+
- Firefox 89+
- Safari 14.1+

Requires WebAssembly and ES6 module support.

## Future Enhancements

- [ ] Multiple cassette management
- [ ] Request history/favorites
- [ ] Export/import response data
- [ ] WebSocket relay connection
- [ ] Audio feedback for operations
- [ ] Dark/light theme toggle