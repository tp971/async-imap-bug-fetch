# async-imap-bug-fetch

This repository demonstrates a bug in `async-imap v0.6.0` when FETCHing messages.
The bug first occured when trying to fetch my personal mailbox:
when trying to fetch one specific mail, the fetch method did not return any mail
and it also corrupts the session, i.e. any further command will return an error.

To reproduce, clone this repository, execute `cargo run` in the `mock-server` folder.
Then, open a second terminal and execute `cargo run` in the `client` folder.

The project in `mock-server` starts a TLS socket on 127.0.0.1:13337
and acts as a minimal IMAP server to reproduce the bug.
Each `FETCH` command will return a valid e-mail,
while making the returned e-mail bigger depending on the FETCHed id.

The project in `client` connects to the server at 127.0.0.1:13337
and attempts to fetch messages 1 to 20 one by one.
Here is where the bug occurs: when fetching message 13,
no message is returned and the session is corrupted,
so when trying to fetch the next message, an error is returned.
