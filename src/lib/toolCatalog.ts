import type { Tool } from '@/stores/ToolStore.ts'
import type { IconName } from '@/components/ui/Icon.tsx'

export const TOOL_CATALOG: Record<Tool, { label: string; icon: IconName }> = {
  select: { label: 'Select', icon: 'select' },
  edit: { label: 'Edit text and images', icon: 'edit' },
  addText: { label: 'Add text', icon: 'text-add' },
  addImage: { label: 'Add image', icon: 'image-add' },
  highlight: { label: 'Highlight text', icon: 'text-highlight' },
  note: { label: 'Sticky note', icon: 'sticky-note' },
  textBox: { label: 'Text box', icon: 'text' },
  ink: { label: 'Draw freehand', icon: 'draw' },
  rectangle: { label: 'Rectangle', icon: 'rectangle-horiz' },
  ellipse: { label: 'Ellipse', icon: 'circle' },
  arrow: { label: 'Arrow', icon: 'arrow-head-tool' },
  stamp: { label: 'Stamp', icon: 'stamp-clone' },
  redactText: { label: 'Redact text', icon: 'text-strike-through' },
  redactArea: { label: 'Redact area', icon: 'select-rectangle' },
  form: { label: 'Prepare form', icon: 'check-box' },
  link: { label: 'Add link', icon: 'link' },
  sign: { label: 'Place signature', icon: 'pen-brush' },
}
