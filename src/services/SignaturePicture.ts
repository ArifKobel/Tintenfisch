import type { SignatureMarkRequest } from '@/types/protocol.ts'

const LONGEST_SIDE = 800

export class SignaturePicture {
  static async read(file: Blob): Promise<SignatureMarkRequest> {
    const bitmap = await createImageBitmap(file)
    try {
      const shrink = Math.min(1, LONGEST_SIDE / Math.max(bitmap.width, bitmap.height))
      const width = Math.max(1, Math.round(bitmap.width * shrink))
      const height = Math.max(1, Math.round(bitmap.height * shrink))
      const context = new OffscreenCanvas(width, height).getContext('2d')
      if (!context) throw new Error('the picture cannot be read in this browser')
      context.drawImage(bitmap, 0, 0, width, height)
      const pixels = context.getImageData(0, 0, width, height).data
      return { kind: 'image', width, height, rgba: new Uint8Array(pixels.buffer, pixels.byteOffset, pixels.byteLength) }
    } finally {
      bitmap.close()
    }
  }
}
