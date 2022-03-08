# `lssaid`, LiSt Steam AppID

A small utility to match a Steam AppID to the corresponding name.

# Installation

Clone or download the repository, and then run `cargo install --path .` in the directory.
Please note that `cargo install` installs the built binaries in `~/.cargo/bin` so make sure that is in your `$PATH`.

# Usage

Here are some common use cases:
- `lssaid`, list the current files and match them using their filenames. Useful for example in a steam library compatdata folder.
- `lssaid -i 440 620`, only output the names corresponding to the specified ids.
- `lssaid -s Portal`, search for steam apps with "Portal" in their name. This is case insensitive.

# Not a valid Steam AppID\*!

The id may be completely invalid, or it can be a tool (like Proton), and is not included in the Steam app list provided by the public API.
