import { describe, expect, it } from 'vitest'
import { PageRange } from '@printing/PageRange.ts'

describe('PageRange', () => {
  it('reads single pages, spans and open ends in the order given, once each', () => {
    expect(PageRange.parse('1-3, 5', 9)).toEqual([0, 1, 2, 4])
    expect(PageRange.parse('8-', 9)).toEqual([7, 8])
    expect(PageRange.parse('-2', 9)).toEqual([0, 1])
    expect(PageRange.parse('4, 2, 4', 9)).toEqual([3, 1])
    expect(PageRange.parse(' 3 - 4 ,', 9)).toEqual([2, 3])
  })

  it('refuses pages the document does not have and text that is no range', () => {
    expect(PageRange.parse('0', 9)).toBeNull()
    expect(PageRange.parse('8-10', 9)).toBeNull()
    expect(PageRange.parse('5-2', 9)).toBeNull()
    expect(PageRange.parse('two', 9)).toBeNull()
    expect(PageRange.parse('', 9)).toBeNull()
  })
})
