# Space-TOML
## What is this?
A formatting-preserving TOML decoder/encoder.

## Why would I use this
To preserve the layout of your files when programmatically changing parts of your TOML configuration files. This means that all your comments are preserved, and that the data will mostly be laid out as before modification. This has the added benefit of being friendlier to version control.

# Source code layout
**tokens.rs**
This contains the lexer, lexer tokens and lexing errors.
**structure.rs**
This contains all the data structures created by the parser, including the tables that the client gets to work with.
**parser.rs**
This contains the parsing logic and errors.
**debug.rs**
This contains methods for pretty-printing error locations in the text.

# Contributing
Contributions are welcome, even if they are just small improvements to documentation, or issues for features/bugs.
Please just fork it and create a pull request.

# License
MIT/Apache-2, like Rust itself.
