park(1)

# NAME

park - Configuration-based dotfiles manager

# SYNOPSIS

*park* [_OPTIONS_] [_TAGS_|_TARGET FILTERS_] < _input_

# DESCRIPTION

*park* is a CLI tool for managing dotfiles based on a configuration file
written in TOML. It does so by symlinking your local dotfiles to the respective
places described in the configuration file.

By default, it won't do anything but print a preview tree of how your dotfiles
will look like according to the given configuration passed via _stdin_.

If everything in the preview tree looks good to you, then it's just a matter
of running the command again but this time with the appropriate flag:

	*park --link < input*

Note that, by default, *park* only successfully executes the linking step
if no problems are detected during the analysis step (the one that generates
the preview tree). Some statuses can be worked around by passing additional
flags, while others are not avoidable and require manual intervention in
the host system for *park* to work. See more details in the _OPTIONS_ section.

# OPTIONS

*-l*, *--link*
	Execute the linking step.

	If any problems are detected during analysis, the linking step will
	be aborted and all problematic files will be listed.

*-r*, *--replace*
	Replace mismatched symlinks.

	This allows bypassing the _MISMATCH_ status by forcing the existing
	symlink to be replaced.

*-c*, *--create-dirs*
	Create parent directories when needed.

	This will prevent links with status UNPARENTED to return an error
	during the linking step by creating all necessary directories that
	compose the symlink's path.

*-h*, *--help*
	Show help usage.

	Note that -h shows a short help, while --help shows a long one.

*-v*, *--version*
	Show version.

# TAGS

Targets can be guarded by tags. Such targets are not evaluated unless their
respective tags are passed as arguments. These tags need to be prepended
with a plus sign:

	*park +tag1 +tag2* < input

Note that tags do not deactivate targets. Their sole purpose is to activate
targets on demand.

The resulting set of tags that *park* uses is a union of tags passed as
arguments with tags set in the configuration file.

# TARGET FILTERS

When arguments don't have a plus sign prepended to them, they serve as
target filters. When one or more filters are passed as arguments, *park*
only evaluates targets whose names match such filters:

	*park +tag1 target1* < input

Note that target filters can be mixed with tags.

# TARGET STATUSES

## READY
The target file is ready to be symlinked

## DONE
The target is already symlinked accordingly

## UNPARENTED
The target file is ready to be symlinked but its parent directory will be
created by *park* during linking

## MISMATCH
A symlink exists, but it points to a different target file

## CONFLICT
Another file already exists where the symlink would be created

## OBSTRUCTED
The parent path of the symlink is not a directory

# SEE ALSO

_park_(5)

# AUTHORS

Developed and maintained by Gabriel Sanches <gabriel@gsr.dev>.

Source code is located at <https://git.sr.ht/~gbrlsnchs/park>.
