# belong

Static blog generator inspired by [mdBook](https://github.com/rust-lang/mdBook).

## Features

- Markdown to HTML rendering.
- TOML front matter.
- Familiar template language base on Jinja2/Django
  ([Tera](https://tera.netlify.com/)).
- Basic theme and templates provided out of the box.
- Syntax highlighting.

## Getting started

Install `belong` using `Cargo`.
```bash
cargo install belong
```

Initialize a new project.

```bash
mkdir -p blog && cd blog
belong init
```

This will create a `belong` config file (`belong.toml`) and an example Markdown
page in the following directory structure.

```
├── .gitignore
├── belong.toml
└── src
    └── hello-world.md
```

Finally build and open the project in the default web browser.

```
belong build --open
```

It will look something like the following:

<div align="center">
<img
  width="750"
  alt="example"
  src="https://user-images.githubusercontent.com/17109887/78908793-e8e6d300-7a82-11ea-8113-324644315967.png"
>
</div>

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
