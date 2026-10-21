import type { Highlight } from '@/types/protocol.ts'

const REACH = 2

export class TextBlocks {
  static contain(blocks: readonly Highlight[], x: number, y: number): boolean {
    return blocks.some((block) => {
      const [dx, dy] = [x - block.x, y - block.y]
      const [cos, sin] = [Math.cos(block.angle), Math.sin(block.angle)]
      const along = dx * cos + dy * sin
      const across = -dx * sin + dy * cos
      return along >= -REACH && along <= block.width + REACH && across >= -REACH && across <= block.height + REACH
    })
  }
}
