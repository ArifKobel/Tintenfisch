export class FileDownload {
  static offer(name: string, bytes: Uint8Array, type = 'application/pdf') {
    const url = URL.createObjectURL(new Blob([bytes as Uint8Array<ArrayBuffer>], { type }))
    const anchor = document.createElement('a')
    anchor.href = url
    anchor.download = name
    anchor.click()
    setTimeout(() => URL.revokeObjectURL(url), 0)
  }
}
