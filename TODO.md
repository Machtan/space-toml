# Things that should be done
- Implement `Error` for the error types (instead of using `show`)
- Rename things prefixed with 'TOML'
- Implement `Table.find_or_insert_array_of_tables`
- Implement `Table.has_key`?
- Handle parsing of too-big integer and float values
- Handle the transformation of scope path segments from tables to arrays of tables (since either is valid, and more information might be gained later)...?
    Eg. [bob.keys.name]
    Where keys 
    Nowait. It cannot change. Yays
    Gotta look at this later when less tired...
- Handle the formatting of scopes after moving tables with formatted scopes to another path.