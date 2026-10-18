import initializeWasm, { Engine } from '@wasm/tintenfisch_wasm.js'
import wasmUrl from '@wasm/tintenfisch_wasm_bg.wasm?url'
import { EngineHost } from '@engine/EngineHost.ts'
import { WorkerRuntime } from '@engine/WorkerRuntime.ts'
import type { WorkerMessage } from '@engine/protocol.ts'

const scope = self as unknown as DedicatedWorkerGlobalScope

async function startEngine(): Promise<EngineHost> {
  await initializeWasm({ module_or_path: wasmUrl })
  const engine = new Engine()
  await Promise.all(
    Engine.fallbackFontFiles().map(async (fileName) => {
      const response = await fetch(`${import.meta.env.BASE_URL}fonts/${fileName}`)
      if (response.ok) engine.registerFont(fileName, new Uint8Array(await response.arrayBuffer()))
    }),
  )
  return new EngineHost(engine)
}

const channel = new MessageChannel()
const pending: Array<() => void> = []
channel.port1.onmessage = () => pending.shift()?.()

const runtime = new WorkerRuntime(
  startEngine(),
  (response, transfer) => scope.postMessage(response, transfer),
  (resume) => {
    pending.push(resume)
    channel.port2.postMessage(null)
  },
)

scope.onmessage = (event: MessageEvent<WorkerMessage>) => runtime.receive(event.data)
