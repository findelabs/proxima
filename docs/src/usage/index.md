# Proxima Usage

Proxima is, at heart, a layer seven proxy, configured using a simple yaml file, and allows for multi-level path structures.

For example, a simple configuration could look like the following:

- /google -> www.google.com
- /yahoo -> www.yahoo.com

A more complex configuration could also proxy in the following:

- /user1/search -> www.google.com
- /user2/search -> www.duckduckgo.com
