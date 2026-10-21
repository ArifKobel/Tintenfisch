export function pdfDate(date: Date): string {
  const two = (value: number) => String(value).padStart(2, '0')
  return `D:${date.getUTCFullYear()}${two(date.getUTCMonth() + 1)}${two(date.getUTCDate())}${two(date.getUTCHours())}${two(date.getUTCMinutes())}${two(date.getUTCSeconds())}Z`
}
