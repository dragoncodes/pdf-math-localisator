# What this?

A simple tool for translating PDFs with math questions into another language with OpenAI API

# Usage

```
cargo run --release
```

Copy the binary where you need it.

Make sure you have `pdftotext` in PATH as it is an external dependency

```
./pdf-localisator "https://pdfobject.com/pdf/sample.pdf" "german"
```

Note: Be sure to have OPENAPI_KEY in your env
