import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export class DesktopFiles {
  static watch(open: (name: string, bytes: Uint8Array) => Promise<void>): () => void {
    if (!isTauri()) return () => {}
    const openPending = async () => {
      for (const path of await invoke<string[]>('opened_files')) {
        const bytes = await invoke<ArrayBuffer>('read_opened_file', { path }).catch(() => null)
        if (bytes) await open(path.split(/[\\/]/).pop() ?? path, new Uint8Array(bytes))
      }
    }
    const unlisten = listen('opened-files', () => void openPending())
    void openPending()
    return () => void unlisten.then((stop) => stop())
  }
}
