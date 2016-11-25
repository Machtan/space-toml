# Things that should be done
These are bound to be in a mostly-outdated state, so only use them as guidance
- ~~Implement `Error` for the error types (instead of using `show`)~~
    - Undo this (keep show/explain, but don't use that for the Display implementation)
- Rename things prefixed with 'TOML'
- Implement `Table.find_or_insert_array_of_tables`
- Implement `Table.contains_key`?
- Handle parsing of too-big integer and float values
- Handle the formatting of scopes after moving tables with formatted scopes to another path.