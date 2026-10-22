import { isTauri } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { writeFile } from '@tauri-apps/plugin-fs'

export class FileDownload {
  static async offer(name: string, bytes: Uint8Array, type = 'application/pdf') {
    if (isTauri()) {
      const path = await save({ defaultPath: name, filters: [{ name: 'PDF', extensions: ['pdf'] }] })
      if (path) await writeFile(path, bytes)
      return
    }
    const url = URL.createObjectURL(new Blob([bytes as Uint8Array<ArrayBuffer>], { type }))
    const anchor = document.createElement('a')
    anchor.href = url
    anchor.download = name
    anchor.click()
    setTimeout(() => URL.revokeObjectURL(url), 0)
  }
}
