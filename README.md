![Tintenfisch](public/brand/banner.jpg)

# Tintenfisch

A PDF editor that runs entirely in the browser. Edit text in place, manage pages, annotate, fill and build forms, redact, sign and protect documents.

The PDF engine is written from scratch in Rust and compiled to WebAssembly. The UI is React with TypeScript.

## Development

```sh
pnpm install
pnpm dev
```

Engine tests run natively:

```sh
cd engine && cargo test
```

## License

MIT. The bundled Liberation fonts are under the SIL Open Font License (`public/fonts/LICENSE`).
