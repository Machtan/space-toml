# Things that should be done
These are bound to be in a mostly-outdated state, so only use them as guidance
- ~~Implement `Error` for the error types (instead of using `show`)~~
    - Undo this (keep show/explain, but don't use that for the Display implementation)
- ~~Rename things prefixed with 'TOML'~~
- Implement `Table.find_or_insert_array_of_tables`
- ~~Implement `Table.contains_key`?~~
- Handle parsing of too-big integer and float values
- Handle the formatting of scopes after moving tables with formatted scopes to another path.
- The top-level table [table.inline] in example 4 is, um... swapped
    Ah. This is because items are now written based on table, and not on order anymore, so [table.inline] is written after [table.subtable] instead of [x.y.z.w]
- Add 'Scope' as a TableItem and only use it for the top level table
- Make the subtables of a non-inline AoT hold their own scopes
- Pass more information along to the 'write' function (internally at least)
- ~~Add Table.remove~~
- Properly handle insertion of tables and AoTs into Tables.
- Handle AoT in Table.find* methods