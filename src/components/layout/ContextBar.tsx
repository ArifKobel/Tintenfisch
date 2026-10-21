import { ImageControls } from '@/components/images/ImageControls.tsx'
import { LinkControls } from '@/components/links/LinkControls.tsx'
import { RedactionControl } from '@/components/redaction/RedactionControl.tsx'
import { ToolOptions } from '@/components/tools/ToolOptions.tsx'

export function ContextBar() {
  return (
    <div className="absolute top-4 left-1/2 z-10 flex max-w-[calc(100%-160px)] -translate-x-1/2 items-center gap-4 rounded bg-surface px-3 py-1 shadow-float empty:hidden max-md:top-2 max-md:max-w-[calc(100%-16px)] max-md:flex-wrap max-md:gap-2 [&>*+*]:border-l [&>*+*]:border-separator [&>*+*]:pl-4">
      <ToolOptions />
      <ImageControls />
      <LinkControls />
      <RedactionControl />
    </div>
  )
}
