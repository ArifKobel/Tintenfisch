import type { RenderedTile } from '@/types/protocol.ts'

export class CanvasPictureEncoder {
  static readonly encode = async (tile: RenderedTile): Promise<Blob> => {
    const canvas = new OffscreenCanvas(tile.width, tile.height)
    const context = canvas.getContext('2d')
    if (!context) throw new Error('this browser cannot draw pages for printing')
    context.putImageData(new ImageData(new Uint8ClampedArray(tile.rgba.buffer as ArrayBuffer, tile.rgba.byteOffset, tile.rgba.byteLength), tile.width, tile.height), 0, 0)
    return canvas.convertToBlob({ type: 'image/png' })
  }
}
