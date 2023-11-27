# tyedev

![white tyedev logo](./static/tyedev-white.png#gh-dark-mode-only)
![black tyedev logo](./static/tyedev-black.png#gh-light-mode-only)

Create and manage devcontainer configuration.

## Install

The preferred method is to use [cargo binstall](https://github.com/cargo-bins/cargo-binstall).

```shell
$ cargo binstall tyedev
```

Alternatively, download the appropiate [release asset](https://github.com/CodeMan99/tyedev/releases/latest). Then extract and copy as needed.

```shell
$ shasum --check tyedev-*.sha256
$ tar -xzf tyedev-*.tar.gz
$ cp tyedev-*/tyedev ~/.local/bin
```

The last method is to use the [Github CLI](https://github.com/cli/cli) with the [redraw/gh-install](https://github.com/redraw/gh-install) extension.

```shell
$ gh ext install redraw/gh-install
$ gh install CodeMan99/tyedev
```

## Usage

Use `--help` to see help on input flags.

```shell
$ tyedev --help
Create and manage devcontainer configuration

Usage: tyedev [OPTIONS] [COMMAND]

Commands:
  completions  Generate shell auto-complete configuration
  init         Create new devcontainer
  inspect      Display details of a specific feature, template, or collection
  list         Overview of collections
  search       Text search the `id`, `keywords`, and `description` fields of templates or features
  help         Print this message or the help of the given subcommand(s)

Options:
  -p, --pull-index  Pull the index of features & templates
  -v, --verbose...  More output per occurrence
  -q, --quiet...    Less output per occurrence
  -h, --help        Print help
  -V, --version     Print version
```

All of the commands depend on a local copy of the _generated_ [collection index](https://github.com/devcontainers/devcontainers.github.io/blob/gh-pages/_data/collection-index.yml).

```shell
$ tyedev --pull-index --verbose
[2023-11-23T15:28:33.056Z INFO  tyedev] Saved to /home/vscode/.local/share/tyedev/devcontainer-index.json
```

### Features

The `tyedev` application is organized into sub-commands.

#### tyedev init

Use to start a new project. Provide no arguments for the default interactive experience.

```shell
$ tyedev init --help
Create new devcontainer

Usage: tyedev init [OPTIONS]

Options:
  -z, --non-interactive               Avoid interactive prompts
  -s, --attempt-single-file           Write to ".devcontainer.json" when using an `image` type template
  -v, --verbose...                    More output per occurrence
  -q, --quiet...                      Less output per occurrence
  -r, --remove-comments               Strip comments from the generated devcontainer.json
  -t, --template-id <OCI_REF>         Reference to a Template in a supported OCI registry
  -f, --include-features <OCI_REF>    Add the given features, may specify more than once
      --include-deprecated            Include deprecated results when searching
  -w, --workspace-folder <DIRECTORY>  Target workspace for the devcontainer configuration
  -h, --help                          Print help
```

Note that `--remove-comments` is not yet actually supported. A better `jsonc` library would be helpful. May need to write my own.

#### tyedev inspect

Describe all details of a specific template or feature. Use as an aid when editing an existing `devcontainer.json`.

```shell
$ tyedev inspect --help
Display details of a specific feature, template, or collection

Usage: tyedev inspect [OPTIONS] <OCI_REF>

Arguments:
  <OCI_REF>  The `id` to inspect

Options:
  -d, --display-as <FORMAT>  Format for displaying the configuration [default: table] [possible values:
                             table, json, none]
      --install-sh           Read the `install.sh` script of a given feature
  -v, --verbose...           More output per occurrence
  -q, --quiet...             Less output per occurrence
      --show-files           List the filenames of a given feature or template
  -h, --help                 Print help
```

The `--show-files` option exists to assist authors with debugging a missing file problem.

The `--install-sh` option exists for debugging container creation failures.

#### tyedev list

List collections overview. Akin to [containers.dev/collections](https://containers.dev/collections).

```shell
$ tyedev list --help
Overview of collections

Usage: tyedev list [OPTIONS]

Options:
  -C, --collection-id <OCI_REF>  Display a given collection, including features and templates
  -v, --verbose...               More output per occurrence
  -q, --quiet...                 Less output per occurrence
  -h, --help                     Print help
```

With `--collection-id` option display all features or templates for the given collection.

```shell
$ tyedev list -q -C ghcr.io/codeman99/features
Name:          Features by CodeMan99
Maintainer:    Cody Taylor
Contact:       https://github.com/CodeMan99/features/issues
Repository:    https://github.com/CodeMan99/features
OCI Reference: ghcr.io/codeman99/features
┌───┬─────────┬────────────────┬──────────────┬───────────────────────────────────────────────────────────┐
│   │ Type    │ OCI Reference  │ Name         │ Description                                               │
├───┼─────────┼────────────────┼──────────────┼───────────────────────────────────────────────────────────┤
│ 1 │ feature │ ~/circleci-cli │ CircleCI CLI │ Install the CircleCI CLI. Also installs the CircleCI ext+ │
│ 2 │ feature │ ~/exercism-cli │ Exercism CLI │ Install the exercism-cli.                                 │
└───┴─────────┴────────────────┴──────────────┴───────────────────────────────────────────────────────────┘
```

#### tyedev search

Find a [template](https://containers.dev/templates) or [feature](https://containers.dev/features).

```shell
$ tyedev search --help
Text search the `id`, `keywords`, and `description` fields of templates or features

Usage: tyedev search [OPTIONS] <VALUE>

Arguments:
  <VALUE>  The keyword(s) to match

Options:
  -c, --collection <COLLECTION>  Match which section of the index [default: templates] [possible values:
                                 templates, features]
  -d, --display-as <FORMAT>      Format for displaying the results [default: table] [possible values:
                                 table, json]
  -v, --verbose...               More output per occurrence
  -f, --fields <FIELD>           Match only within the given fields [possible values: id, name,
                                 description, keywords]
  -q, --quiet...                 Less output per occurrence
      --include-deprecated       Display deprecated results
  -h, --help                     Print help
```

Example: Find a _feature_ with `circleci-cli` in the _id_ field only, and output as _json_.

```shell
$ tyedev search --quiet -d json -f id -c features circleci-cli | jq '.[1]'
{
  "collection": "Features",
  "id": "ghcr.io/codeman99/features/circleci-cli",
  "version": "1.2.0",
  "name": "CircleCI CLI",
  "description": "Install the CircleCI CLI. Also installs the CircleCI extension for vscode.",
  "keywords": null
}
```

### Non-Features

This project avoids interop with docker or any editor. Please see the [related tools](#related-tools) list to accomplish runtime needs.

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
