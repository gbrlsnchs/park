park(5)

# NAME

park - configuration schema and target statuses

# DESCRIPTION

*park* reads TOML files from _stdin_ in order to decide how to organize
your dotfiles.

Refer to <https://toml.io> for further details about TOML.

# CONFIGURATION SCHEMA

The following fields are top-level fields.

[- *Name*
:- *Type*
:- *Description*
:- *Default*
|  *base_dir*
:  string
:  The path to be used as base directory for symlinks.
:  _Empty string_, which means symlinks will end up in the current working
   directory.
|  *work_dir*
:  string
:  The path to be used as working directory for symlinks.
:  The _current working directory_ is used.
|  *tags*
:  string array
:  List of tags that will be used to evaluate targets. These tags complement
   the ones passed as arguments to *park*.
:  _Empty array_, which means only tags passed arguments will be considered.
|  *targets*
:  _target_ table
:  Targets to be evaluated and symlinked by *park*. See the _target_ section
   for more details.
:  _Empty table_, which means there's nothing for *park* to do.

## target

[- *Name*
:- *Type*
:- *Description*
:- *Default*
|  *link*
:  _link_ table
:  Information about the respective symlink file. See the _link_ section
   for more details.
:  _Empty table_, uses the defaults from _link_.
|  *tags*
:  _tags_ table
:  List of conjuctive and disjunctive tags that guard the target. See the
   _tags_ section for more details.
:  _Empty table_, uses the defaults from _tags_.

## link

[- *Name*
:- *Type*
:- *Description*
:- *Default*
|  *base_dir*
:  string
:  Base directory for the link in particular.
:  _Empty string_, uses the top-level base directory.
|  *name*
:  string
:  The name of the resulting symlink.
:  _Empty string_, uses the target name as the symlink name.

## tags
[- *Name*
:- *Type*
:- *Description*
:- *Default*
|  *all_of*
:  string array
:  List of conjunctive tags that guard the target, that is, all tags listed
   must be passed to *park* for the target to be considered.
:  _Empty array_, which means no conjunctive tags guard the target.
|  *any_of*
:  string array
:  List of disjunctive tags that guard the target, that is, at least one of
   the tags listed must be passed to *park* for the target to be considered.
:  _Empty array_, which means no disjunctive tags guard the target.

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

_park_(1)

# AUTHORS

Developed and maintained by Gabriel Sanches <gabriel@gsr.dev>.

Source code is located at <https://git.sr.ht/~gbrlsnchs/park>.
