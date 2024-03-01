# TODOS

- add hystory to terminal (ncurses?)
- optimize number serialization in opcodes for small numbers (8 and 16 bytes integer numbers)
- Make lexer owning the `Vec<u8>`, Tokens the a reference lifetime equal to the one of the lexer, Compiler::compile consume the lexer
