# devconf

Create and manage devcontainer configuration.

## Work in Progress

This project is absolutely a work in progress. Bugs do exist. More documentation needs to be written. More tests need to be written. General design patterns need to be reviewed.

### Assist

I (CodeMan99) would love any amount of assistance. In particular, general design patterns. I am learning Rust and this project is my first _sharable_ tool in the language.

## Usage

The to see help on input flags.

```shell
$ devconf --help
```

All of the commands depend on a local copy of the _generated_ [collection index](https://github.com/devcontainers/devcontainers.github.io/blob/gh-pages/_data/collection-index.yml).

```shell
$ devconf --pull-index
```

### Features

...

#### devconf init

Use to start a new project. Provide no arguments for the default interactive experience. Use `--help` to learn what can be provided as arguments.

Note that `--remove-comments` is not yet actually supported. A better `jsonc` library would be helpful. May need to write my own.

#### devconf inspect

Describe all details of a specific template or feature. The `id` is a required argument. Use as an aid when editing an existing `devcontainer.json`.

#### devconf list

List collections overview. With `--collection-id` option display all features or templates for the given collection.

#### devconf search

Find a template or feature from the official [collections](https://containers.dev/collections).

### Non-Features

This project avoids interop with docker or any editor.

## Contributing

A _devcontainer_ exists for this project. There are some permissions errors that need to be sorted out for `/usr/local/cargo`. Feel free to ask questions.

## Related Tools

- `devcontainer` - [Official CLI](https://github.com/devcontainers/cli) tool. Primary use is building and executing containers.
- `devcontainerx` - [Unofficial CLI](https://github.com/stuartleeks/devcontainer-cli) tool.
- `vscli` - A CLI tool to [launch vscode projects](https://github.com/michidk/vscli), which supports devcontainers.
- `devcon` - [Start devcontainers without vscode](https://github.com/guitsaru/devcon).
- As well as other [supporting tools](https://containers.dev/supporting) in the devcontainer ecosystem.
