# tyedev

![white tyedev logo](./static/tyedev-white.png#gh-dark-mode-only)
![black tyedev logo](./static/tyedev-black.png#gh-light-mode-only)

Create and manage devcontainer configuration.

## Usage

The to see help on input flags.

```shell
$ tyedev --help
```

All of the commands depend on a local copy of the _generated_ [collection index](https://github.com/devcontainers/devcontainers.github.io/blob/gh-pages/_data/collection-index.yml).

```shell
$ tyedev --pull-index
```

### Features

...

#### tyedev init

Use to start a new project. Provide no arguments for the default interactive experience. Use `--help` to learn what can be provided as arguments.

Note that `--remove-comments` is not yet actually supported. A better `jsonc` library would be helpful. May need to write my own.

#### tyedev inspect

Describe all details of a specific template or feature. The `id` is a required argument. Use as an aid when editing an existing `devcontainer.json`.

#### tyedev list

List collections overview. With `--collection-id` option display all features or templates for the given collection.

#### tyedev search

Find a template or feature from the official [collections](https://containers.dev/collections).

### Non-Features

This project avoids interop with docker or any editor.

## Work in Progress

This project is absolutely a work in progress. Bugs do exist. More documentation needs to be written. More tests need to be written. General design patterns need to be reviewed.

### Contributing

How to help!

- :wrench: Just use the tool. :speaking_head: Feedback is appreciated.
- :bug: Report bugs.
- :book: Improve documentation.
- :computer: Contribute code directly.

### Devcontainer

For code contributions please use the _devcontainer_ for this project.

There are some permissions errors that need to be sorted out for `/usr/local/cargo`. Upstream issues have already been filed. For now just correct this manually.

```shell
$ sudo chmod -R g+w $CARGO_HOME
```

## Related Tools

- `devcontainer` - [Official CLI](https://github.com/devcontainers/cli) tool. Primary use is building and executing containers.
- `devcontainerx` - [Unofficial CLI](https://github.com/stuartleeks/devcontainer-cli) to improve the experience of working with Visual Studio Code devcontainers.
- `vscli` - A CLI tool to [launch vscode projects](https://github.com/michidk/vscli), which supports devcontainers.
- `devcon` - [Start devcontainers without vscode](https://github.com/guitsaru/devcon).
- `devopen` - Simple [bash function](https://gist.github.com/CodeMan99/852d8539bd35a347a48d4a6119ff70e7) to open a devcontaienr project from a WSL directory.
- As well as other [supporting tools](https://containers.dev/supporting) in the devcontainer ecosystem.

## General Devcontainers Resources

- The [VSCode Overview](https://code.visualstudio.com/docs/devcontainers/containers) documentation.
- The [awesome-devcontainers](https://github.com/manekinekko/awesome-devcontainers) repository.
