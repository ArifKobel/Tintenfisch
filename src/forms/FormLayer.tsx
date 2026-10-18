import { FieldDesigner } from '@forms/FieldDesigner.tsx'
import { FormControl } from '@forms/FormControl.tsx'
import { useFormsState } from '@forms/useFormsState.ts'
import { useToolState } from '@tools/useToolState.ts'

export function FormLayer({ pageId, cssScale }: { pageId: string; cssScale: number }) {
  const { fields } = useFormsState()
  const { tool } = useToolState()
  if (tool === 'form') return <FieldDesigner pageId={pageId} cssScale={cssScale} />
  if (tool !== 'select') return null
  return (
    <div className="form-layer">
      {fields.flatMap((field) =>
        field.widgets
          .filter((widget) => widget.pageId === pageId)
          .map((widget) => <FormControl key={widget.id} field={field} widget={widget} cssScale={cssScale} />),
      )}
    </div>
  )
}
