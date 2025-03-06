import { finalizeEvent, generateSecretKey, getPublicKey } from 'nostr-tools/pure'
import { Relay } from 'nostr-tools/relay'
import { SimplePool } from 'nostr-tools/pool'
import WebSocket from 'ws'
import { useWebSocketImplementation } from 'nostr-tools/pool'

useWebSocketImplementation(WebSocket)

class RelayHandler {
    private pool: SimplePool
    private relayUrls: string[]
    private subscriptions: any[] = []
    private reconnectInterval?: ReturnType<typeof setTimeout>

    constructor(relayUrls: string[]) {
        this.pool = new SimplePool()
        this.relayUrls = relayUrls
        this.startReconnectLoop()
    }

    private startReconnectLoop() {
        this.reconnectInterval = setInterval(() => {
            this.relayUrls.forEach((url) => {
                const normalizedUrl = new URL(url).href
                if (!this.pool.listConnectionStatus().get(normalizedUrl)) {
                    this.ensureRelay(url)
                }
            })
        }, 10000)
    }

    private async ensureRelay(url: string) {
        try {
            await this.pool.ensureRelay(url, { connectionTimeout: 5000 })
            console.log(`Connected to relay: ${url}`)
        } catch (error) {
            console.log(`Failed to connect to relay ${url}:`, error)
        }
    }

    async publishEvent(event: any): Promise<void> {
        try {
            await Promise.any(this.pool.publish(this.relayUrls, event))
            console.log(`Event published(${event.kind}), id: ${event.id.slice(0, 12)}`)
        } catch (error) {
            console.error('Failed to publish event:', error)
            throw error
        }
    }

    subscribeToEvent(eventId: string, onEvent: (event: any) => void) {
        const sub = this.pool.subscribeMany(
            this.relayUrls,
            [
                {
                    kinds: [5910, 6910, 7000],
                    since: Math.floor(Date.now() / 1000)
                },
            ],
            {
                onevent(event) {
                    console.log(`Event received(${event.kind}), id: ${event.id.slice(0, 12)}`)
                    onEvent(event)
                },
                oneose() {
                    console.log('Reached end of stored events')
                },
                onclose(reason) {
                    console.log('Subscription closed:', reason)
                }
            }
        )
        this.subscriptions.push(sub)
        return sub
    }

    cleanup() {
        if (this.reconnectInterval) {
            clearInterval(this.reconnectInterval)
        }
        this.subscriptions.forEach((sub) => sub.close())
        this.subscriptions = []
        this.pool.close(this.relayUrls)
    }
}

async function main() {
    const relays = ['wss://relay.nostr.net', 'wss://relay.damus.io', 'wss://ithurtswhenip.ee', 'wss://relay.snort.social', 'wss://relay.nostr.band']
    const handler = new RelayHandler(relays)

    // Create a subscription to listen for responses
    const sk = generateSecretKey()
    const pk = getPublicKey(sk)
    console.log('pk', pk)

    // Create a tool request event
    const toolRequest = {
        kind: 5910,
        created_at: Math.floor(Date.now() / 1000),
        tags: [
            ['c', 'execute-tool']
            // ['c', 'list-tools']
        ],
        content: JSON.stringify({
            name: "plusone",
            parameters: {
                a: 1,
            }
        })
    }

    // Sign the event
    const signedRequest = finalizeEvent(toolRequest, sk)

    console.dir(signedRequest, { depth: null })

    // Set up subscription before publishing
    handler.subscribeToEvent(signedRequest.id, (event) => {
        // if(event.kind !== 5910 && event.kind !== 6190) return 
        try {
            // console.log(event.kind)
            console.dir(event, { depth: null })
            // const result = JSON.parse(event.content)
            // console.log('Tool response:', result.content)
        } catch (e) {
            console.error('Failed to parse event content:', e)
        }
    })

    // Now publish the event
    console.log('Publishing event:', signedRequest.id)
    await handler.publishEvent(signedRequest)

    // Keep the process running
    console.log('Waiting for responses...')
    process.on('SIGINT', () => {
        console.log('Cleaning up...')
        handler.cleanup()
        process.exit()
    })
}

main().catch(console.error)



