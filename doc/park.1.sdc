park(1)

{{data}}

# TAGS

Targets can be guarded by tags. Such targets are not evaluated unless their
respective tags are passed as arguments. These tags need to be prepended
with a plus sign:

	*park* +tag1 +tag2

Note that tags do not deactivate targets. Their sole purpose is to activate
targets on demand.

The resulting set of tags that *park* uses is a union of tags passed as
arguments with tags set in the configuration file.

# TARGET FILTERING

When arguments don't have a plus sign prepended to them, they serve as
target filters. When one or more filters are passed as arguments, *park*
only evaluates targets whose names match such filters:

	*park* target1 target2

Note that target filters can be mixed with tags.

# SEE ALSO

_park_(5)

# AUTHORS

Developed and maintained by Gabriel Sanches <gabriel@gsr.dev>.

Source code is located at <https://git.sr.ht/~gbrlsnchs/park>.
