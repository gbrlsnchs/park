# Park

## About
Park is a configuration-based dotfiles manager. It reads TOMLs files directly from stdin in order to
understand your dotfiles structure.

## Example
This is a small extract adapted from my own dotfiles. The file below is named `configs.toml`:

```toml
base_dir = "$XDG_CONFIG_HOME"
tags = ["fontconfig"]

[targets.neovim]
link.name = "nvim"

[targets.fontconfig]
tags.all_of = ["fontconfig"]

[targets."zsh/config/zshrc"]
link.base_dir = "$HOME"
link.name = ".zshrc"

[targets.bspwm]
tags.all_of = ["xorg", "bspwm"]
```

Then, with the help of `envsubst`, we can feed Park's stdin with environment variables properly
substituted:
```console
$ envsubst < config.toml | park
.                 := /home/you/dotfiles
├── fontconfig    <- /home/you/.config/fontconfig (READY)
├── neovim        <- /home/you/.config/nvim       (DONE)
└── zsh
    └── config
        └── zshrc <- /home/you/.zshrc             (READY)
```

Then, in order to create their respective symlinks, we need to pass `--link` to the command:
```console
$ envsubst < config.toml | park --link
```

So if all goes well, exit code is 0 and all necessary symlinks are created.

### Tags
If you pay enough attention to the output, `bspwm` was not evaluated. That's because it's guarded by
two conjunctive tags (`xorg` and `bspwm`), that is, both tags must be passed to the command so that
the target gets evaluated. In order to do so, we need to pass those tags as arguments to Park:
```console
$ envsubst < config.toml | park xorg bspwm
.                 := /home/you/dotfiles
├── bspwm         <- /home/you/.config/bspwm      (READY)
├── fontconfig    <- /home/you/.config/fontconfig (DONE)
├── neovim        <- /home/you/.config/nvim       (DONE)
└── zsh
    └── config
        └── zshrc <- /home/you/.zshrc             (DONE)
```

## Configuration
The configuration consists of the following fields:

- `base_dir` (string, required): Tells Park which base directory for symlinks
- `work_dir` (string, optional): Tells Park which base directory to use for targets (your dotfiles),
  defaults to current directory
- `tags` (array of strings, optional): This denotes tags that will always be on while running Park
- `targets` (object, optional): 
	- `link` (object, optional):
		- `base_dir` (string, optional): Base directory for the symlink to be created
		- `name` (string, optional): Custom symlink name, defaults to the target basename
	- `tags` (object, optional):
		- `all_of` (array of strings, optional): Represents conjunctive tags and, if not empty, all
		  tags in it need to be provided by the user in order for the target to be evaluated
		- `any_of` (array of strings, optional): Represents disjunctive tags and, if not empty, at
		  least one of its tags needs to be provided in order for the target to be evaluated
